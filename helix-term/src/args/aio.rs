use anyhow::{bail, Context, Result};
use helix_core::{Selection, Transaction};
use helix_view::{document::Mode, DocumentId, Editor, ViewId};
use ignore::{DirEntry, WalkBuilder};
use once_cell::sync::Lazy;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Output},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 128 * 1024 * 1024;
const MAX_DIFF_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
enum AioSource {
    Files,
    GitDiff { revision: String },
}

#[derive(Clone)]
struct AioFile {
    id: usize,
    relative: String,
    original_relative: Option<String>,
    original_hash: Option<u64>,
    deleted: bool,
    wrapped: bool,
}

#[derive(Clone)]
struct PendingAio {
    root: PathBuf,
    bootstrap: PathBuf,
    token: String,
    columns: Option<usize>,
    highlight_extension: String,
    source: AioSource,
    files: Vec<AioFile>,
}

#[derive(Clone)]
struct ActiveAio {
    root: PathBuf,
    token: String,
    doc: DocumentId,
    highlight_extension: String,
    syntax_suspended: bool,
    source: AioSource,
    files: Vec<AioFile>,
}

pub(crate) struct PreparedAio {
    pub root: PathBuf,
    pub bootstrap: PathBuf,
}

pub(crate) struct AioActivation {
    pub columns: Option<usize>,
    pub files: usize,
    pub git_revision: Option<String>,
}

pub(crate) struct AioSaveResult {
    pub written: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub files: usize,
}

static PENDING: Lazy<Mutex<Option<PendingAio>>> = Lazy::new(|| Mutex::new(None));
static ACTIVE: Lazy<Mutex<Option<ActiveAio>>> = Lazy::new(|| Mutex::new(None));

fn pending() -> Option<PendingAio> {
    PENDING
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn active() -> Option<ActiveAio> {
    ACTIVE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn set_active(value: Option<ActiveAio>) {
    *ACTIVE.lock().unwrap_or_else(|error| error.into_inner()) = value;
}

fn ensure_files_mode(active: &ActiveAio) -> Result<()> {
    if matches!(active.source, AioSource::GitDiff { .. }) {
        bail!("Git diff view is for review only; open a filesystem AIO view to edit files");
    }
    Ok(())
}

fn git_command(root: &Path, arguments: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .with_context(|| "could not run Git; make sure git.exe is installed and in PATH")
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<Vec<u8>> {
    let output = git_command(root, arguments)?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        bail!("Git command failed: {}", details.trim());
    }
    Ok(output.stdout)
}

fn git_repository_root(directory: &Path) -> Result<PathBuf> {
    let root = git_output(directory, &["rev-parse", "--show-toplevel"])?;
    let root = String::from_utf8(root).context("Git repository root is not valid UTF-8")?;
    let root = root.trim();
    fs::canonicalize(root).with_context(|| format!("could not resolve Git repository root {root}"))
}

fn resolve_commit(root: &Path, revision: &str) -> Result<String> {
    if revision.is_empty() || revision.starts_with('-') {
        bail!("invalid Git revision: {revision}");
    }
    let expression = format!("{revision}^{{commit}}");
    let bytes = git_output(
        root,
        &["rev-parse", "--verify", "--end-of-options", &expression],
    )?;
    let resolved = String::from_utf8(bytes).context("Git returned a non-UTF-8 commit ID")?;
    Ok(resolved.trim().to_string())
}

fn git_diff_contents(root: &Path, revision: &str) -> Result<String> {
    let options = [
        "--no-ext-diff",
        "--no-color",
        "--no-textconv",
        "--find-renames",
    ];

    let bytes = if revision == "HEAD" {
        let mut arguments = vec!["diff"];
        arguments.extend_from_slice(&options);
        arguments.extend_from_slice(&["HEAD", "--"]);
        git_output(root, &arguments)?
    } else {
        let (parent, commit) = if revision == "HEAD^" {
            (
                resolve_commit(root, "HEAD^")?,
                resolve_commit(root, "HEAD")?,
            )
        } else {
            let commit = resolve_commit(root, revision)?;
            let parent_expression = format!("{commit}^1");
            let parent = resolve_commit(root, &parent_expression);
            match parent {
                Ok(parent) => (parent, commit),
                Err(_) => {
                    let mut arguments = vec!["show"];
                    arguments.extend_from_slice(&options);
                    arguments.extend_from_slice(&["--format=", "--root", &commit, "--"]);
                    let output = git_output(root, &arguments)?;
                    if output.len() > MAX_DIFF_BYTES {
                        bail!("Git diff is larger than 16 MiB; narrow the selection");
                    }
                    let text = String::from_utf8_lossy(&output).into_owned();
                    return Ok(if text.trim().is_empty() {
                        format!("No changes for {revision}\n")
                    } else {
                        text
                    });
                }
            }
        };
        let mut arguments = vec!["diff"];
        arguments.extend_from_slice(&options);
        arguments.extend_from_slice(&[parent.as_str(), commit.as_str(), "--"]);
        git_output(root, &arguments)?
    };

    if bytes.len() > MAX_DIFF_BYTES {
        bail!("Git diff is larger than 16 MiB; narrow the selection");
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(if text.trim().is_empty() {
        format!("No changes for {revision}\n")
    } else {
        text
    })
}

pub(crate) fn prepare_git_diff(
    directory: &Path,
    revision: &str,
    columns: Option<usize>,
) -> Result<PreparedAio> {
    if pending().is_some() {
        bail!("an AIO workspace is already pending");
    }
    let root = git_repository_root(directory)?;
    let contents = git_diff_contents(&root, revision)?;
    let token = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let bootstrap = create_bootstrap(&token, "diff", &contents)?;

    *PENDING.lock().unwrap_or_else(|error| error.into_inner()) = Some(PendingAio {
        root: root.clone(),
        bootstrap: bootstrap.clone(),
        token,
        columns,
        highlight_extension: "diff".to_string(),
        source: AioSource::GitDiff {
            revision: revision.to_string(),
        },
        files: Vec::new(),
    });

    Ok(PreparedAio { root, bootstrap })
}

fn allow_entry(entry: &DirEntry) -> bool {
    !matches!(
        entry.file_name().to_str(),
        Some(".git" | ".pijul" | ".jj" | ".hg" | ".svn")
    )
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn begin_marker(_token: &str, id: usize, relative: &str) -> String {
    format!("// ╭─ {id} · {relative}")
}

fn end_marker(_token: &str, id: usize) -> String {
    format!("// ╰─ {id} ─╯")
}

fn extension_of(relative: &str) -> Option<String> {
    Path::new(relative)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

fn javascript_family(extension: &str) -> bool {
    matches!(
        extension,
        "ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs"
    )
}

fn should_wrap(relative: &str, bytes: &[u8], highlight_extension: &str) -> bool {
    if bytes.windows(2).any(|window| window == b"*/") {
        return false;
    }

    let Some(extension) = extension_of(relative) else {
        return javascript_family(highlight_extension);
    };

    if javascript_family(highlight_extension) {
        !javascript_family(&extension)
    } else {
        extension != highlight_extension
    }
}

fn nearest_project_root(path: &Path) -> PathBuf {
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    for ancestor in absolute.ancestors() {
        if ancestor.join(".git").exists()
            || ancestor.join(".jj").exists()
            || ancestor.join(".svn").exists()
            || ancestor.join(".helix").exists()
        {
            return ancestor.to_path_buf();
        }
    }

    absolute
}

fn scan(root: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(false)
        .parents(true)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(allow_entry)
        .add_custom_ignore_filename(helix_loader::config_dir().join("ignore"))
        .add_custom_ignore_filename(".helix/ignore");

    let mut files = Vec::new();
    let mut total = 0usize;

    for entry in builder.build() {
        let entry = entry.context("failed while scanning AIO workspace")?;
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
        {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if metadata.len() > MAX_FILE_BYTES {
            continue;
        }

        let bytes = match fs::read(entry.path()) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };

        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            continue;
        }

        total = total.saturating_add(bytes.len());
        if total > MAX_TOTAL_BYTES {
            bail!("AIO workspace exceeds the 128 MiB text limit");
        }

        let relative = match entry.path().strip_prefix(root) {
            Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        files.push((relative, bytes));
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));

    if files.is_empty() {
        bail!("AIO found no UTF-8 text files under {}", root.display());
    }

    Ok(files)
}

fn render_files(
    token: &str,
    files: &[AioFile],
    contents: &HashMap<usize, Vec<u8>>,
) -> Result<String> {
    let mut aggregate = String::new();

    for file in files.iter().filter(|file| !file.deleted) {
        let bytes = contents
            .get(&file.id)
            .with_context(|| format!("missing AIO contents for {}", file.relative))?;
        let text = std::str::from_utf8(bytes)
            .with_context(|| format!("AIO contents became non-UTF-8 for {}", file.relative))?;

        aggregate.push_str(&begin_marker(token, file.id, &file.relative));
        aggregate.push('\n');

        if file.wrapped {
            aggregate.push_str("/*\n");
        }

        aggregate.push_str(text);
        aggregate.push('\n');

        if file.wrapped {
            aggregate.push_str("*/\n");
        }

        aggregate.push_str(&end_marker(token, file.id));
        aggregate.push_str("\n\n");
    }

    Ok(aggregate)
}

fn render(active: &ActiveAio, contents: &HashMap<usize, Vec<u8>>) -> Result<String> {
    render_files(&active.token, &active.files, contents)
}

fn preferred_extension(files: &[(String, Vec<u8>)]) -> String {
    let mut scores = HashMap::<String, (usize, usize)>::new();
    let mut has_ts = false;
    let mut has_tsx = false;
    let mut has_js = false;
    let mut has_jsx = false;

    for (relative, bytes) in files {
        let Some(extension) = Path::new(relative)
            .extension()
            .and_then(|value| value.to_str())
        else {
            continue;
        };

        let extension = extension.to_ascii_lowercase();

        if extension.is_empty()
            || extension.len() > 12
            || !extension
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            continue;
        }

        match extension.as_str() {
            "tsx" => has_tsx = true,
            "ts" | "mts" | "cts" => has_ts = true,
            "jsx" => has_jsx = true,
            "js" | "mjs" | "cjs" => has_js = true,
            _ => {}
        }

        let score = scores.entry(extension).or_insert((0, 0));
        score.0 += 1;
        score.1 = score.1.saturating_add(bytes.len());
    }

    if has_tsx || (has_ts && has_jsx) {
        return "tsx".to_string();
    }

    if has_ts {
        return "ts".to_string();
    }

    if has_jsx {
        return "jsx".to_string();
    }

    if has_js {
        return "js".to_string();
    }

    scores
        .into_iter()
        .max_by(|left, right| {
            left.1
                 .0
                .cmp(&right.1 .0)
                .then_with(|| left.1 .1.cmp(&right.1 .1))
                .then_with(|| right.0.cmp(&left.0))
        })
        .map(|(extension, _)| extension)
        .unwrap_or_else(|| "txt".to_string())
}

fn create_bootstrap(token: &str, extension: &str, aggregate: &str) -> Result<PathBuf> {
    let temp = std::env::temp_dir();

    for attempt in 0..1000usize {
        let path = temp.join(format!("helix-aio-{token}-{attempt}.{extension}"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(aggregate.as_bytes())
                    .with_context(|| format!("failed to write {}", path.display()))?;
                return Ok(helix_stdx::path::canonicalize(path));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    bail!("could not create the temporary AIO bootstrap")
}

fn find_view(editor: &Editor, doc: DocumentId) -> Option<ViewId> {
    editor
        .tree
        .views()
        .find_map(|(view, _)| (view.doc == doc).then_some(view.id))
}

fn commit_document(editor: &mut Editor, doc_id: DocumentId, clean: bool) -> Result<()> {
    let view_id = find_view(editor, doc_id).context("AIO view no longer exists")?;
    let (tree, documents) = (&mut editor.tree, &mut editor.documents);
    let view = tree.get_mut(view_id);
    let doc = documents
        .get_mut(&doc_id)
        .context("AIO document no longer exists")?;
    doc.append_changes_to_history(view);
    if clean {
        doc.reset_modified();
    }
    Ok(())
}

fn replace_document(
    editor: &mut Editor,
    doc_id: DocumentId,
    text: String,
    clean: bool,
    cursor_char: Option<usize>,
) -> Result<()> {
    let view_id = find_view(editor, doc_id).context("AIO view no longer exists")?;
    let transaction = {
        let doc = editor
            .documents
            .get(&doc_id)
            .context("AIO document no longer exists")?;
        Transaction::change(
            doc.text(),
            std::iter::once((0, doc.text().len_chars(), Some(text.into()))),
        )
    };

    let view_ids = editor
        .tree
        .views()
        .filter_map(|(view, _)| (view.doc == doc_id).then_some(view.id))
        .collect::<Vec<_>>();

    {
        let doc = editor
            .documents
            .get_mut(&doc_id)
            .context("AIO document no longer exists")?;
        doc.apply(&transaction, view_id);

        if let Some(cursor_char) = cursor_char {
            let point = cursor_char.min(doc.text().len_chars());
            for view_id in view_ids {
                doc.set_selection(view_id, Selection::point(point));
            }
        }
    }

    commit_document(editor, doc_id, clean)
}

fn extract_contents(active: &ActiveAio, text: &str) -> Result<HashMap<usize, Vec<u8>>> {
    let mut contents = HashMap::new();
    let mut cursor = 0usize;

    for file in active.files.iter().filter(|file| !file.deleted) {
        let begin_marker = begin_marker(&active.token, file.id, &file.relative);
        let end_marker = end_marker(&active.token, file.id);
        let begin = format!("{begin_marker}\n");
        let end = format!("\n{end_marker}");

        if text.matches(&begin_marker).count() != 1 || text.matches(&end_marker).count() != 1 {
            bail!("AIO boundary was edited for {}", file.relative);
        }

        let begin_offset = text[cursor..]
            .find(&begin)
            .map(|offset| cursor + offset)
            .with_context(|| format!("AIO boundary was edited for {}", file.relative))?;

        let mut content_start = begin_offset + begin.len();

        if file.wrapped {
            let wrapper = "/*\n";
            if !text[content_start..].starts_with(wrapper) {
                bail!("AIO wrapper was edited for {}", file.relative);
            }
            content_start += wrapper.len();
        }

        let end_pattern = if file.wrapped {
            format!("\n*/{}", end)
        } else {
            end.clone()
        };

        let end_offset = text[content_start..]
            .find(&end_pattern)
            .map(|offset| content_start + offset)
            .with_context(|| format!("AIO boundary was edited for {}", file.relative))?;

        contents.insert(file.id, text.as_bytes()[content_start..end_offset].to_vec());
        cursor = end_offset + end_pattern.len();
    }

    Ok(contents)
}

fn current_file_id(editor: &Editor, active: &ActiveAio) -> Result<usize> {
    let view_id = editor.tree.focus;
    let view = editor
        .tree
        .try_get(view_id)
        .context("current AIO view no longer exists")?;

    if view.doc != active.doc {
        bail!("current view is not the AIO workspace");
    }

    let doc = editor
        .documents
        .get(&active.doc)
        .context("AIO document no longer exists")?;
    let cursor_char = doc
        .selection(view_id)
        .primary()
        .cursor(doc.text().slice(..));
    let cursor_byte = doc.text().char_to_byte(cursor_char);
    let text = doc.text().to_string();

    for file in active.files.iter().filter(|file| !file.deleted) {
        let begin = begin_marker(&active.token, file.id, &file.relative);
        let end = end_marker(&active.token, file.id);
        let Some(begin_offset) = text.find(&begin) else {
            continue;
        };
        let Some(end_relative) = text[begin_offset..].find(&end) else {
            continue;
        };
        let end_offset = begin_offset + end_relative + end.len();

        if (begin_offset..=end_offset).contains(&cursor_byte) {
            return Ok(file.id);
        }
    }

    bail!("cursor is not inside an AIO file section")
}

fn resolve_relative(root: &Path, input: &str) -> Result<(String, PathBuf)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("file path may not be empty");
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        bail!("AIO file paths must be relative to the workspace root");
    }

    let mut clean = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("AIO file path may not leave the workspace root")
            }
        }
    }

    if clean.as_os_str().is_empty() {
        bail!("file path may not be empty");
    }

    let relative = clean.to_string_lossy().replace('\\', "/");
    Ok((relative, root.join(clean)))
}

fn path_for(root: &Path, relative: &str) -> PathBuf {
    root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR))
}

pub(crate) fn prepare(
    root: &Path,
    columns: Option<usize>,
    discover_project_root: bool,
) -> Result<PreparedAio> {
    if pending().is_some() {
        bail!("an AIO workspace is already pending");
    }

    let root = if discover_project_root {
        nearest_project_root(root)
    } else {
        fs::canonicalize(root)
            .with_context(|| format!("could not resolve AIO root {}", root.display()))?
    };

    let scanned = scan(&root)?;
    let bootstrap_extension = preferred_extension(&scanned);
    let token = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let mut files = Vec::with_capacity(scanned.len());
    let mut contents = HashMap::new();

    for (id, (relative, bytes)) in scanned.into_iter().enumerate() {
        let hash = hash_bytes(&bytes);
        let wrapped = should_wrap(&relative, &bytes, &bootstrap_extension);
        files.push(AioFile {
            id,
            relative: relative.clone(),
            original_relative: Some(relative),
            original_hash: Some(hash),
            deleted: false,
            wrapped,
        });
        contents.insert(id, bytes);
    }

    let aggregate = render_files(&token, &files, &contents)?;
    let bootstrap = create_bootstrap(&token, &bootstrap_extension, &aggregate)?;

    *PENDING.lock().unwrap_or_else(|error| error.into_inner()) = Some(PendingAio {
        root: root.clone(),
        bootstrap: bootstrap.clone(),
        token,
        columns,
        highlight_extension: bootstrap_extension,
        source: AioSource::Files,
        files,
    });

    Ok(PreparedAio { root, bootstrap })
}

pub(crate) fn activate_pending(editor: &mut Editor) -> Option<AioActivation> {
    let pending = pending()?;

    let doc = editor.documents.iter().find_map(|(id, document)| {
        (document.path() == Some(pending.bootstrap.as_path())).then_some(*id)
    })?;

    let pending = PENDING
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()?;

    if let Some(document) = editor.documents.get_mut(&doc) {
        let language_server_names = document
            .language_config()
            .map(|config| {
                config
                    .language_servers
                    .iter()
                    .map(|features| features.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for name in language_server_names {
            document.remove_language_server_by_name(&name);
        }

        document.reset_modified();
    }

    let _ = fs::remove_file(&pending.bootstrap);

    let file_count = pending.files.len();

    set_active(Some(ActiveAio {
        root: pending.root,
        token: pending.token,
        doc,
        highlight_extension: pending.highlight_extension,
        syntax_suspended: false,
        source: pending.source.clone(),
        files: pending.files,
    }));

    Some(AioActivation {
        columns: pending.columns,
        files: file_count,
        git_revision: match pending.source {
            AioSource::Files => None,
            AioSource::GitDiff { revision } => Some(revision),
        },
    })
}

pub(crate) fn save_active(editor: &mut Editor) -> Result<Option<AioSaveResult>> {
    let Some(mut active) = active() else {
        return Ok(None);
    };

    ensure_files_mode(&active)?;

    let focused_doc = editor.tree.try_get(editor.tree.focus).map(|view| view.doc);
    if focused_doc != Some(active.doc) {
        return Ok(None);
    }

    let text = editor
        .documents
        .get(&active.doc)
        .context("AIO document no longer exists")?
        .text()
        .to_string();

    let contents = extract_contents(&active, &text)?;
    let mut original_disk = HashMap::<usize, Option<Vec<u8>>>::new();

    for file in &active.files {
        if let Some(original_relative) = &file.original_relative {
            let original_path = path_for(&active.root, original_relative);
            let disk = match fs::read(&original_path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };

            if !file.deleted {
                let content = contents
                    .get(&file.id)
                    .with_context(|| format!("missing AIO contents for {}", file.relative))?;

                match (&disk, file.original_hash) {
                    (Some(bytes), Some(hash))
                        if hash_bytes(bytes) != hash && bytes.as_slice() != content.as_slice() =>
                    {
                        bail!(
                            "AIO conflict: {} changed on disk since the workspace was opened",
                            original_relative
                        );
                    }
                    (None, _) => {
                        bail!("AIO conflict: {} disappeared on disk", original_relative);
                    }
                    _ => {}
                }
            }

            original_disk.insert(file.id, disk);
        } else {
            original_disk.insert(file.id, None);
        }

        if !file.deleted {
            let destination = path_for(&active.root, &file.relative);
            let original_path = file
                .original_relative
                .as_ref()
                .map(|relative| path_for(&active.root, relative));

            if destination.exists()
                && original_path
                    .as_ref()
                    .is_none_or(|original| *original != destination)
            {
                bail!("AIO destination already exists: {}", file.relative);
            }
        }
    }

    let mut written = 0usize;
    let mut deleted = 0usize;
    let mut renamed = 0usize;

    for file in active.files.iter().filter(|file| !file.deleted) {
        let content = contents
            .get(&file.id)
            .with_context(|| format!("missing AIO contents for {}", file.relative))?;
        let destination = path_for(&active.root, &file.relative);

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let needs_write = match original_disk.get(&file.id).and_then(|value| value.as_ref()) {
            Some(bytes) => {
                bytes.as_slice() != content.as_slice()
                    || file.original_relative.as_deref() != Some(file.relative.as_str())
            }
            None => true,
        };

        if needs_write {
            fs::write(&destination, content)
                .with_context(|| format!("failed to write {}", destination.display()))?;
            written += 1;
        }
    }

    for file in &active.files {
        let Some(original_relative) = &file.original_relative else {
            continue;
        };

        let original_path = path_for(&active.root, original_relative);

        if file.deleted {
            if original_path.exists() {
                fs::remove_file(&original_path)
                    .with_context(|| format!("failed to delete {}", original_path.display()))?;
                deleted += 1;
            }
        } else if original_relative != &file.relative {
            if original_path.exists() {
                fs::remove_file(&original_path)
                    .with_context(|| format!("failed to remove {}", original_path.display()))?;
            }
            renamed += 1;
        }
    }

    active.files.retain(|file| !file.deleted);

    for file in &mut active.files {
        let content = contents
            .get(&file.id)
            .with_context(|| format!("missing AIO contents for {}", file.relative))?;
        file.original_relative = Some(file.relative.clone());
        file.original_hash = Some(hash_bytes(content));
    }

    let files = active.files.len();
    set_active(Some(active.clone()));

    if let Some(document) = editor.documents.get_mut(&active.doc) {
        document.reset_modified();
    }

    Ok(Some(AioSaveResult {
        written,
        deleted,
        renamed,
        files,
    }))
}

pub(crate) fn sync_syntax_mode(editor: &mut Editor) {
    let insert_mode = editor.mode() == Mode::Insert;
    let mut aio = ACTIVE.lock().unwrap_or_else(|error| error.into_inner());
    let Some(current) = aio.as_mut() else {
        return;
    };

    if insert_mode {
        if current.syntax_suspended {
            return;
        }

        let Some(document) = editor.documents.get_mut(&current.doc) else {
            return;
        };

        if document.syntax.is_some() {
            document.syntax = None;
            current.syntax_suspended = true;
        }

        return;
    }

    if !current.syntax_suspended {
        return;
    }

    let loader = editor.syn_loader.load();

    if let Some(document) = editor.documents.get_mut(&current.doc) {
        document.detect_language(&loader);
    }

    current.syntax_suspended = false;
}

pub(crate) fn git_revision() -> Option<String> {
    active().and_then(|current| match current.source {
        AioSource::Files => None,
        AioSource::GitDiff { revision } => Some(revision),
    })
}

pub(crate) fn current_relative(editor: &Editor) -> Result<String> {
    let active = active().context("AIO mode is not active")?;
    ensure_files_mode(&active)?;
    let id = current_file_id(editor, &active)?;
    active
        .files
        .iter()
        .find(|file| file.id == id && !file.deleted)
        .map(|file| file.relative.clone())
        .context("current AIO file no longer exists")
}

pub(crate) fn new_file(editor: &mut Editor, input: &str) -> Result<String> {
    let mut active = active().context("AIO mode is not active")?;
    ensure_files_mode(&active)?;
    let (relative, path) = resolve_relative(&active.root, input)?;

    if path.exists()
        || active
            .files
            .iter()
            .any(|file| !file.deleted && file.relative == relative)
    {
        bail!("AIO file already exists: {relative}");
    }

    let text = editor
        .documents
        .get(&active.doc)
        .context("AIO document no longer exists")?
        .text()
        .to_string();

    let mut contents = extract_contents(&active, &text)?;
    let id = active
        .files
        .iter()
        .map(|file| file.id)
        .max()
        .map_or(0, |id| id + 1);

    let wrapped = should_wrap(&relative, &[], &active.highlight_extension);
    active.files.push(AioFile {
        id,
        relative: relative.clone(),
        original_relative: None,
        original_hash: None,
        deleted: false,
        wrapped,
    });
    contents.insert(id, Vec::new());

    let aggregate = render(&active, &contents)?;
    let marker = format!("{}\n", begin_marker(&active.token, id, &relative));
    let cursor_byte = aggregate
        .find(&marker)
        .map(|offset| offset + marker.len())
        .unwrap_or(aggregate.len());
    let cursor_char = aggregate[..cursor_byte].chars().count();

    set_active(Some(active.clone()));
    replace_document(editor, active.doc, aggregate, false, Some(cursor_char))?;

    Ok(relative)
}

pub(crate) fn delete_current(editor: &mut Editor) -> Result<String> {
    let mut active = active().context("AIO mode is not active")?;
    ensure_files_mode(&active)?;
    let id = current_file_id(editor, &active)?;

    let text = editor
        .documents
        .get(&active.doc)
        .context("AIO document no longer exists")?
        .text()
        .to_string();

    let mut contents = extract_contents(&active, &text)?;
    let index = active
        .files
        .iter()
        .position(|file| file.id == id)
        .context("current AIO file no longer exists")?;
    let relative = active.files[index].relative.clone();

    if active.files[index].original_relative.is_none() {
        active.files.remove(index);
    } else {
        active.files[index].deleted = true;
    }

    contents.remove(&id);
    let aggregate = render(&active, &contents)?;
    set_active(Some(active.clone()));
    replace_document(editor, active.doc, aggregate, false, None)?;

    Ok(relative)
}

pub(crate) fn rename_current(editor: &mut Editor, input: &str) -> Result<(String, String)> {
    let mut active = active().context("AIO mode is not active")?;
    ensure_files_mode(&active)?;
    let id = current_file_id(editor, &active)?;
    let (relative, destination) = resolve_relative(&active.root, input)?;

    if active
        .files
        .iter()
        .any(|file| !file.deleted && file.id != id && file.relative == relative)
    {
        bail!("AIO file already exists: {relative}");
    }

    let index = active
        .files
        .iter()
        .position(|file| file.id == id)
        .context("current AIO file no longer exists")?;
    let old = active.files[index].relative.clone();
    let original = active.files[index]
        .original_relative
        .as_ref()
        .map(|relative| path_for(&active.root, relative));

    if destination.exists() && original.as_ref().is_none_or(|path| *path != destination) {
        bail!("AIO destination already exists: {relative}");
    }

    let text = editor
        .documents
        .get(&active.doc)
        .context("AIO document no longer exists")?
        .text()
        .to_string();
    let contents = extract_contents(&active, &text)?;

    active.files[index].relative = relative.clone();
    let content = contents.get(&id).map(Vec::as_slice).unwrap_or_default();
    active.files[index].wrapped = should_wrap(&relative, content, &active.highlight_extension);
    let aggregate = render(&active, &contents)?;

    set_active(Some(active.clone()));
    replace_document(editor, active.doc, aggregate, false, None)?;

    Ok((old, relative))
}

pub(crate) fn refresh(editor: &mut Editor) -> Result<usize> {
    let mut active = active().context("AIO mode is not active")?;

    if editor
        .documents
        .get(&active.doc)
        .context("AIO document no longer exists")?
        .is_modified()
    {
        bail!("AIO has unsaved edits; discard them before refreshing");
    }

    if let AioSource::GitDiff { revision } = &active.source {
        let contents = git_diff_contents(&active.root, revision)?;
        replace_document(editor, active.doc, contents, true, None)?;
        return Ok(0);
    }

    let scanned = scan(&active.root)?;
    let mut contents = HashMap::new();
    active.files.clear();

    for (id, (relative, bytes)) in scanned.into_iter().enumerate() {
        let wrapped = should_wrap(&relative, &bytes, &active.highlight_extension);
        active.files.push(AioFile {
            id,
            relative: relative.clone(),
            original_relative: Some(relative),
            original_hash: Some(hash_bytes(&bytes)),
            deleted: false,
            wrapped,
        });
        contents.insert(id, bytes);
    }

    let aggregate = render(&active, &contents)?;
    let files = active.files.len();
    set_active(Some(active.clone()));
    replace_document(editor, active.doc, aggregate, true, None)?;

    Ok(files)
}
