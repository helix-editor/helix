use std::{
    cmp::Ordering,
    collections::HashSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use helix_core::{Rope, Selection, Transaction};
use helix_view::{
    directory_buffer::{DirectoryBufferState, DirectoryEntry, DirectorySort},
    Document, DocumentId, Editor, ViewId,
};
use ignore::WalkBuilder;
use imara_diff::{Algorithm, Diff, InternedInput};

use crate::filter_picker_entry;

#[derive(Clone)]
struct DesiredEntry {
    relative_path: PathBuf,
    is_dir: bool,
}

struct RenameOp {
    from: PathBuf,
    to: PathBuf,
}

struct StringLines<'a>(&'a [String]);

impl<'a> imara_diff::TokenSource for StringLines<'a> {
    type Token = String;
    type Tokenizer = std::iter::Cloned<std::slice::Iter<'a, String>>;

    fn tokenize(&self) -> Self::Tokenizer {
        self.0.iter().cloned()
    }

    fn estimate_tokens(&self) -> u32 {
        self.0.len() as u32
    }
}

fn get_excluded_types() -> ignore::types::Types {
    use ignore::types::TypesBuilder;
    let mut type_builder = TypesBuilder::new();
    type_builder
        .add(
            "compressed",
            "*.{zip,gz,bz2,zst,lzo,sz,tgz,tbz2,lz,lz4,lzma,lzo,z,Z,xz,7z,rar,cab}",
        )
        .expect("invalid type definition");
    type_builder.negate("all");
    type_builder
        .build()
        .expect("failed to build excluded_types")
}

pub fn open_directory_buffer(
    editor: &mut Editor,
    root: PathBuf,
    action: helix_view::editor::Action,
) -> Result<DocumentId> {
    let root = helix_stdx::path::canonicalize(root);
    if !root.exists() {
        bail!("Directory does not exist");
    }
    if !root.is_dir() {
        bail!("Path is not a directory");
    }

    let config = editor.config();
    let state = DirectoryBufferState {
        root: root.clone(),
        entries: read_directory_entries(
            &root,
            editor,
            config.file_manager.show_hidden,
            config.file_manager.sort,
        )?,
        show_hidden: config.file_manager.show_hidden,
        sort: config.file_manager.sort,
        delete_to_trash: config.file_manager.delete_to_trash,
    };

    let text = render_directory_buffer(&state);
    let mut doc = Document::from(
        Rope::from(text),
        None,
        editor.config.clone(),
        editor.syn_loader.clone(),
    );
    doc.set_path(Some(&root));

    let id = editor.open_directory_buffer(doc, state, action);
    Ok(id)
}

pub fn refresh_directory_buffer(editor: &mut Editor, doc_id: DocumentId) -> Result<()> {
    let state = editor
        .directory_buffer_state(doc_id)
        .cloned()
        .ok_or_else(|| anyhow!("Not a directory buffer"))?;

    let entries = read_directory_entries(&state.root, editor, state.show_hidden, state.sort)?;
    let mut new_state = state;
    new_state.entries = entries;

    replace_directory_buffer_contents(editor, doc_id, &new_state)?;
    editor.set_directory_buffer_state(doc_id, new_state);
    Ok(())
}

pub fn apply_directory_buffer(editor: &mut Editor, doc_id: DocumentId) -> Result<()> {
    let state = editor
        .directory_buffer_state(doc_id)
        .cloned()
        .ok_or_else(|| anyhow!("Not a directory buffer"))?;

    let doc = editor
        .documents
        .get(&doc_id)
        .ok_or_else(|| anyhow!("Missing document"))?;
    let text = doc.text().to_string();
    let new_entries = parse_buffer_entries(&text)?;

    let old_display: Vec<String> = state
        .entries
        .iter()
        .map(|entry| display_name(&entry.relative_path, entry.is_dir))
        .collect();
    let new_display: Vec<String> = new_entries
        .iter()
        .map(|entry| display_name(&entry.relative_path, entry.is_dir))
        .collect();

    let old_set: HashSet<String> = old_display.iter().cloned().collect();
    let new_set: HashSet<String> = new_display.iter().cloned().collect();
    if old_set == new_set {
        refresh_directory_buffer(editor, doc_id)?;
        if let Some(doc) = editor.documents.get_mut(&doc_id) {
            doc.reset_modified();
        }
        return Ok(());
    }

    let file = InternedInput::new(StringLines(&old_display), StringLines(&new_display));
    let diff = Diff::compute(Algorithm::Myers, &file);

    let mut renames: Vec<RenameOp> = Vec::new();
    let mut deletes: Vec<PathBuf> = Vec::new();
    let mut creates: Vec<DesiredEntry> = Vec::new();

    for hunk in diff.hunks() {
        let old_chunk = &state.entries[hunk.before.start as usize..hunk.before.end as usize];
        let new_chunk = &new_entries[hunk.after.start as usize..hunk.after.end as usize];
        let common = old_chunk.len().min(new_chunk.len());

        for idx in 0..common {
            let old = &old_chunk[idx];
            let new = &new_chunk[idx];
            if old.is_dir != new.is_dir {
                bail!(
                    "Cannot change entry type: '{}' vs '{}'",
                    display_name(&old.relative_path, old.is_dir),
                    display_name(&new.relative_path, new.is_dir)
                );
            }
            if old.relative_path != new.relative_path {
                renames.push(RenameOp {
                    from: state.root.join(&old.relative_path),
                    to: state.root.join(&new.relative_path),
                });
            }
        }

        if old_chunk.len() > common {
            for old in &old_chunk[common..] {
                deletes.push(state.root.join(&old.relative_path));
            }
        }

        if new_chunk.len() > common {
            creates.extend(new_chunk[common..].iter().cloned());
        }
    }

    let rename_sources: HashSet<PathBuf> = renames.iter().map(|op| op.from.clone()).collect();

    for path in deletes {
        if rename_sources.contains(&path) {
            continue;
        }
        delete_path(&path, state.delete_to_trash)?;
    }

    perform_renames(&renames)?;

    for entry in creates {
        create_entry(&state.root, &entry)?;
    }

    refresh_directory_buffer(editor, doc_id)?;

    if let Some(doc) = editor.documents.get_mut(&doc_id) {
        doc.reset_modified();
    }

    Ok(())
}

pub fn toggle_hidden(editor: &mut Editor, doc_id: DocumentId) -> Result<()> {
    let (root, show_hidden, sort) = {
        let state = editor
            .directory_buffer_state(doc_id)
            .ok_or_else(|| anyhow!("Not a directory buffer"))?;
        (state.root.clone(), !state.show_hidden, state.sort)
    };
    let new_entries = read_directory_entries(&root, editor, show_hidden, sort)?;
    {
        let state = editor
            .directory_buffer_state_mut(doc_id)
            .ok_or_else(|| anyhow!("Not a directory buffer"))?;
        state.show_hidden = show_hidden;
        state.entries = new_entries;
    }
    let state = editor
        .directory_buffer_state(doc_id)
        .cloned()
        .ok_or_else(|| anyhow!("Not a directory buffer"))?;
    replace_directory_buffer_contents(editor, doc_id, &state)?;
    Ok(())
}

pub fn change_sort(editor: &mut Editor, doc_id: DocumentId) -> Result<()> {
    let (root, show_hidden, sort) = {
        let state = editor
            .directory_buffer_state(doc_id)
            .ok_or_else(|| anyhow!("Not a directory buffer"))?;
        let next_sort = match state.sort {
            DirectorySort::TypeThenNameAsc => DirectorySort::TypeThenNameDesc,
            DirectorySort::TypeThenNameDesc => DirectorySort::NameAsc,
            DirectorySort::NameAsc => DirectorySort::NameDesc,
            DirectorySort::NameDesc => DirectorySort::TypeThenNameAsc,
        };
        (state.root.clone(), state.show_hidden, next_sort)
    };
    let new_entries = read_directory_entries(&root, editor, show_hidden, sort)?;
    {
        let state = editor
            .directory_buffer_state_mut(doc_id)
            .ok_or_else(|| anyhow!("Not a directory buffer"))?;
        state.sort = sort;
        state.entries = new_entries;
    }
    let state = editor
        .directory_buffer_state(doc_id)
        .cloned()
        .ok_or_else(|| anyhow!("Not a directory buffer"))?;
    replace_directory_buffer_contents(editor, doc_id, &state)?;
    Ok(())
}

pub fn set_root(editor: &mut Editor, doc_id: DocumentId, root: PathBuf) -> Result<()> {
    let root = helix_stdx::path::canonicalize(root);
    if !root.exists() {
        bail!("Directory does not exist");
    }
    if !root.is_dir() {
        bail!("Path is not a directory");
    }
    let (show_hidden, sort) = {
        let state = editor
            .directory_buffer_state(doc_id)
            .ok_or_else(|| anyhow!("Not a directory buffer"))?;
        (state.show_hidden, state.sort)
    };
    let entries = read_directory_entries(&root, editor, show_hidden, sort)?;
    {
        let state = editor
            .directory_buffer_state_mut(doc_id)
            .ok_or_else(|| anyhow!("Not a directory buffer"))?;
        state.root = root.clone();
        state.entries = entries;
    }
    let state = editor
        .directory_buffer_state(doc_id)
        .cloned()
        .ok_or_else(|| anyhow!("Not a directory buffer"))?;
    replace_directory_buffer_contents(editor, doc_id, &state)?;
    if let Some(doc) = editor.documents.get_mut(&doc_id) {
        doc.set_path(Some(&root));
    }
    Ok(())
}

pub fn current_entry_path(
    editor: &Editor,
    doc_id: DocumentId,
    view_id: ViewId,
) -> Result<Option<(PathBuf, bool)>> {
    let state = match editor.directory_buffer_state(doc_id) {
        Some(state) => state,
        None => return Ok(None),
    };
    let doc = editor
        .documents
        .get(&doc_id)
        .ok_or_else(|| anyhow!("Missing document"))?;
    let selection = doc.selection(view_id).primary();
    let cursor = selection.cursor(doc.text().slice(..));
    let line = doc.text().char_to_line(cursor);
    let line_text = doc
        .text()
        .line(line)
        .to_string()
        .trim_end_matches(['\r', '\n'])
        .to_string();

    if is_parent_line(&line_text) {
        let parent = state.root.parent().map(|path| path.to_path_buf());
        return Ok(parent.map(|path| (path, true)));
    }

    if line_text.is_empty() {
        return Ok(None);
    }

    let (relative, is_dir) = parse_line_entry(&line_text)?;
    Ok(Some((state.root.join(relative), is_dir)))
}

fn read_directory_entries(
    root: &Path,
    editor: &Editor,
    show_hidden: bool,
    sort: DirectorySort,
) -> Result<Vec<DirectoryEntry>> {
    let config = editor.config();
    let mut walk_builder = WalkBuilder::new(root);
    let mut entries: Vec<DirectoryEntry> = walk_builder
        .hidden(!show_hidden)
        .parents(config.file_explorer.parents)
        .ignore(config.file_explorer.ignore)
        .follow_links(config.file_explorer.follow_symlinks)
        .git_ignore(config.file_explorer.git_ignore)
        .git_global(config.file_explorer.git_global)
        .git_exclude(config.file_explorer.git_exclude)
        .max_depth(Some(1))
        .add_custom_ignore_filename(helix_loader::config_dir().join("ignore"))
        .add_custom_ignore_filename(".helix/ignore")
        .types(get_excluded_types())
        .build()
        .filter_map(|entry| {
            entry
                .map(|entry| {
                    if !filter_picker_entry(&entry, root, false) {
                        return None;
                    }
                    let path = entry.path();
                    if path == root {
                        return None;
                    }
                    let is_dir = path.is_dir();
                    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
                    Some(DirectoryEntry {
                        relative_path,
                        is_dir,
                    })
                })
                .ok()
                .flatten()
        })
        .collect();

    sort_entries(&mut entries, sort);

    Ok(entries)
}

fn sort_entries(entries: &mut [DirectoryEntry], sort: DirectorySort) {
    let name = |entry: &DirectoryEntry| entry.relative_path.to_string_lossy().to_string();
    match sort {
        DirectorySort::TypeThenNameAsc => entries.sort_by(|a, b| {
            let type_ord = (!a.is_dir).cmp(&!b.is_dir);
            if type_ord == Ordering::Equal {
                name(a).cmp(&name(b))
            } else {
                type_ord
            }
        }),
        DirectorySort::TypeThenNameDesc => entries.sort_by(|a, b| {
            let type_ord = (!a.is_dir).cmp(&!b.is_dir);
            if type_ord == Ordering::Equal {
                name(b).cmp(&name(a))
            } else {
                type_ord
            }
        }),
        DirectorySort::NameAsc => entries.sort_by_key(&name),
        DirectorySort::NameDesc => entries.sort_by_key(|entry| std::cmp::Reverse(name(entry))),
    }
}

fn render_directory_buffer(state: &DirectoryBufferState) -> String {
    let mut lines = Vec::new();
    if state.root.parent().is_some() {
        lines.push(parent_line());
    }
    for entry in &state.entries {
        lines.push(display_name(&entry.relative_path, entry.is_dir));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn replace_directory_buffer_contents(
    editor: &mut Editor,
    doc_id: DocumentId,
    state: &DirectoryBufferState,
) -> Result<()> {
    let text = render_directory_buffer(state);
    let view_id = editor.get_synced_view_id(doc_id);
    let len = {
        let doc = editor
            .documents
            .get(&doc_id)
            .ok_or_else(|| anyhow!("Missing document"))?;
        doc.text().len_chars()
    };
    let transaction = Transaction::change(
        doc!(editor, &doc_id).text(),
        std::iter::once((0, len, Some(text.into()))),
    )
    .with_selection(Selection::point(0));
    {
        let doc = doc_mut!(editor, &doc_id);
        doc.apply(&transaction, view_id);
    }
    {
        let doc = doc_mut!(editor, &doc_id);
        let view = view_mut!(editor, view_id);
        doc.append_changes_to_history(view);
        doc.reset_modified();
    }
    Ok(())
}

fn parse_buffer_entries(text: &str) -> Result<Vec<DesiredEntry>> {
    let mut entries = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (idx, raw_line) in text.split_terminator('\n').enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if is_parent_line(line) {
            continue;
        }
        let (relative, is_dir) =
            parse_line_entry(line).with_context(|| format!("Invalid entry on line {}", idx + 1))?;
        if relative.as_os_str() == OsStr::new(".") || relative.as_os_str() == OsStr::new("..") {
            bail!("Invalid entry on line {}: {}", idx + 1, line);
        }
        let key = format!(
            "{}:{}",
            relative.to_string_lossy(),
            if is_dir { "dir" } else { "file" }
        );
        if !seen.insert(key) {
            bail!("Duplicate entry on line {}: {}", idx + 1, line);
        }
        entries.push(DesiredEntry {
            relative_path: relative,
            is_dir,
        });
    }

    Ok(entries)
}

fn parse_line_entry(line: &str) -> Result<(PathBuf, bool)> {
    let mut is_dir = false;
    let mut name = line.to_string();
    if let Some(stripped) = strip_dir_suffix(&name) {
        name = stripped.to_string();
        is_dir = true;
    }
    if name.is_empty() {
        bail!("Empty entry");
    }
    Ok((PathBuf::from(name), is_dir))
}

fn strip_dir_suffix(name: &str) -> Option<&str> {
    if name.ends_with(std::path::MAIN_SEPARATOR) {
        return name.strip_suffix(std::path::MAIN_SEPARATOR);
    }
    if name.ends_with('/') {
        return name.strip_suffix('/');
    }
    if name.ends_with('\\') {
        return name.strip_suffix('\\');
    }
    None
}

fn display_name(relative: &Path, is_dir: bool) -> String {
    let mut name = relative.to_string_lossy().to_string();
    if is_dir && !name.ends_with(std::path::MAIN_SEPARATOR) {
        name.push(std::path::MAIN_SEPARATOR);
    }
    name
}

fn parent_line() -> String {
    format!("..{}", std::path::MAIN_SEPARATOR)
}

fn is_parent_line(line: &str) -> bool {
    line == ".." || line == "../" || line == "..\\" || line == parent_line()
}

fn delete_path(path: &Path, delete_to_trash: bool) -> Result<()> {
    if delete_to_trash {
        trash::delete(path).with_context(|| format!("Failed to trash {}", path.display()))?;
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    } else {
        fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn create_entry(root: &Path, entry: &DesiredEntry) -> Result<()> {
    let path = root.join(&entry.relative_path);
    if path.exists() {
        bail!("Path already exists: {}", path.display());
    }
    if entry.is_dir {
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create directory {}", path.display()))?;
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("Failed to create file {}", path.display()))?;
    Ok(())
}

fn perform_renames(renames: &[RenameOp]) -> Result<()> {
    if renames.is_empty() {
        return Ok(());
    }

    let sources: HashSet<PathBuf> = renames.iter().map(|op| op.from.clone()).collect();
    let mut temp_moves: Vec<(PathBuf, PathBuf)> = Vec::new();

    for op in renames {
        if op.from == op.to {
            continue;
        }
        if op.to.exists() && !sources.contains(&op.to) {
            bail!("Target already exists: {}", op.to.to_string_lossy());
        }
    }

    for op in renames {
        if op.from == op.to {
            continue;
        }
        if sources.contains(&op.to) {
            let temp = unique_temp_path(&op.from)?;
            fs::rename(&op.from, &temp).with_context(|| {
                format!(
                    "Failed to rename {} -> {}",
                    op.from.display(),
                    temp.display()
                )
            })?;
            temp_moves.push((temp, op.to.clone()));
        } else {
            fs::rename(&op.from, &op.to).with_context(|| {
                format!(
                    "Failed to rename {} -> {}",
                    op.from.display(),
                    op.to.display()
                )
            })?;
        }
    }

    for (temp, to) in temp_moves {
        fs::rename(&temp, &to)
            .with_context(|| format!("Failed to rename {} -> {}", temp.display(), to.display()))?;
    }

    Ok(())
}

fn unique_temp_path(source: &Path) -> Result<PathBuf> {
    let parent = source
        .parent()
        .ok_or_else(|| anyhow!("Invalid path: {}", source.display()))?;
    let base = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("entry");
    for idx in 0..1000 {
        let candidate = parent.join(format!(".hx-oil-tmp-{}-{}", base, idx));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!(
        "Unable to find a temporary filename for {}",
        source.display()
    )
}
