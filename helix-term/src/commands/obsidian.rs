use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use anyhow::{bail, ensure, Context as _};
use helix_core::{
    command_line::Args, coords_at_pos, pos_at_coords, str_utils::char_to_byte_idx, Position,
    Selection, Tendril, Transaction,
};
use helix_stdx::path::expand_tilde;
use helix_view::editor::{Action, ObsidianConfig};
use ignore::WalkBuilder;

use crate::compositor::Context as CompositorContext;
use crate::job;
use crate::ui::{self, overlay::overlaid, Picker, PickerColumn, PromptEvent};

struct NoteItem {
    path: PathBuf,
    relative: String,
}

struct SearchItem {
    path: PathBuf,
    relative: String,
    line: usize,
    text: String,
}

struct BacklinkItem {
    path: PathBuf,
    relative: String,
    line: usize,
    text: String,
}

struct TemplateItem {
    path: PathBuf,
    relative: String,
}

struct LinkItem {
    path: Option<PathBuf>,
    target: String,
    line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkKind {
    Wiki,
    Markdown,
}

#[derive(Debug, Clone)]
struct Link {
    kind: LinkKind,
    target: String,
    start: usize,
    end: usize,
}

/// Notes opened through Obsidian actions during this session. The map is
/// session-only and deliberately does not touch the shared file picker order.
fn recent_notes() -> &'static Mutex<HashMap<PathBuf, Instant>> {
    static RECENT: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();
    RECENT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn recency_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn mark_note_recent(path: &Path) {
    recent_notes()
        .lock()
        .unwrap()
        .insert(recency_key(path), Instant::now());
}

fn note_recency() -> HashMap<PathBuf, Instant> {
    recent_notes().lock().unwrap().clone()
}

fn order_quick_switch_items(
    items: Vec<NoteItem>,
    recency: &HashMap<PathBuf, Instant>,
) -> Vec<NoteItem> {
    let mut keyed: Vec<(Option<Instant>, NoteItem)> = items
        .into_iter()
        .map(|item| (recency.get(&recency_key(&item.path)).copied(), item))
        .collect();
    keyed.sort_by(|(a_time, a), (b_time, b)| {
        b_time
            .cmp(a_time)
            .then_with(|| a.relative.to_lowercase().cmp(&b.relative.to_lowercase()))
    });
    keyed.into_iter().map(|(_, item)| item).collect()
}

pub fn quick_switch(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let templates = templates_dir(&vault, &config);
    let paths = note_paths(&vault, templates.as_deref());
    ensure!(
        !paths.is_empty(),
        "no markdown notes found in {}",
        vault.display()
    );

    let mut recency = note_recency();
    for doc in cx.editor.documents() {
        if let Some(path) = doc.path() {
            recency
                .entry(recency_key(path))
                .and_modify(|time| *time = (*time).max(doc.focused_at))
                .or_insert(doc.focused_at);
        }
    }

    let items: Vec<NoteItem> = paths
        .into_iter()
        .map(|path| NoteItem {
            relative: relative_path(&vault, &path),
            path,
        })
        .collect();
    let items = order_quick_switch_items(items, &recency);

    push_note_picker(cx, items);
    Ok(())
}

pub fn search(cx: &mut CompositorContext, args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let query = args.join(" ");
    if query.trim().is_empty() {
        push_component(cx, |_editor, compositor| {
            let prompt = ui::Prompt::new(
                "vault search:".into(),
                None,
                ui::completers::none,
                |cx, input, event| {
                    if event != PromptEvent::Validate {
                        return;
                    }
                    if let Err(err) = run_search(cx, input) {
                        cx.editor.set_error(err.to_string());
                    }
                },
            );
            compositor.push(Box::new(prompt));
        });
    } else {
        run_search(cx, &query)?;
    }

    Ok(())
}

pub fn new_note(cx: &mut CompositorContext, args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let title = args.join(" ");
    if title.trim().is_empty() {
        push_component(cx, |_editor, compositor| {
            let prompt = ui::Prompt::new(
                "new note:".into(),
                None,
                ui::completers::none,
                |cx, input, event| {
                    if event != PromptEvent::Validate {
                        return;
                    }
                    if let Err(err) = create_and_open(cx, input) {
                        cx.editor.set_error(err.to_string());
                    }
                },
            );
            compositor.push(Box::new(prompt));
        });
    } else {
        create_and_open(cx, &title)?;
    }

    Ok(())
}

pub fn follow_link(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let (source, line, cursor_col) = current_line(cx)?;
    let link = link_at(&line, cursor_col).context("no wiki or markdown link under cursor")?;

    if link.target.is_empty() {
        // A link like `[[#heading]]` points at the current note.
        open_note(cx, &source, None, Action::Replace);
        return Ok(());
    }

    let mut matches = resolve_link(&vault, &config, &source, &link);
    matches.sort();
    matches.dedup();

    match matches.len() {
        0 => {
            let target = link.target.clone();
            push_component(cx, move |editor, compositor| {
                let prompt = ui::Prompt::new(
                    "create note:".into(),
                    None,
                    ui::completers::none,
                    |cx, input, event| {
                        if event != PromptEvent::Validate {
                            return;
                        }
                        if let Err(err) = create_and_open(cx, input) {
                            cx.editor.set_error(err.to_string());
                        }
                    },
                )
                .with_line(target, editor);
                compositor.push(Box::new(prompt));
            });
        }
        1 => {
            open_note(cx, &matches[0], None, Action::Replace);
        }
        _ => {
            let items = matches
                .into_iter()
                .map(|path| NoteItem {
                    relative: relative_path(&vault, &path),
                    path,
                })
                .collect();
            push_note_picker(cx, items);
        }
    }

    Ok(())
}

pub fn backlinks(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let (source, _, _) = current_line(cx)?;
    let title = note_title(&source);
    let source_canonical = fs::canonicalize(&source).unwrap_or_else(|_| source.clone());
    let templates = templates_dir(&vault, &config);

    let mut items = Vec::new();
    for path in note_paths(&vault, templates.as_deref()) {
        let path_canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if path_canonical == source_canonical {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for (line_idx, line) in text.lines().enumerate() {
            let mut matched = false;
            for link in wiki_links(line).into_iter().chain(markdown_links(line)) {
                let is_match = match link.kind {
                    LinkKind::Wiki => wiki_target_matches(&link.target, &title, &vault, &source),
                    LinkKind::Markdown => resolve_markdown_path(&path, &vault, &link.target)
                        .and_then(|target| fs::canonicalize(target).ok())
                        .is_some_and(|target| target == source_canonical),
                };
                if is_match {
                    matched = true;
                    break;
                }
            }
            if matched {
                items.push(BacklinkItem {
                    path: path.clone(),
                    relative: relative_path(&vault, &path),
                    line: line_idx,
                    text: line.trim_end().to_string(),
                });
            }
        }
    }

    if items.is_empty() {
        cx.editor.set_status(format!("no backlinks to `{}`", title));
        return Ok(());
    }

    items.sort_by(|a, b| {
        a.relative
            .cmp(&b.relative)
            .then_with(|| a.line.cmp(&b.line))
    });
    push_backlink_picker(cx, items);
    Ok(())
}

fn vault_config(cx: &CompositorContext) -> anyhow::Result<(PathBuf, ObsidianConfig)> {
    vault_config_for(cx.editor)
}

fn vault_config_for(editor: &helix_view::Editor) -> anyhow::Result<(PathBuf, ObsidianConfig)> {
    let config = editor.config().obsidian.clone();
    ensure!(
        config.enable,
        "Obsidian integration is disabled; set `[editor.obsidian] enable = true`"
    );
    let vault = config
        .vault
        .as_deref()
        .context("`editor.obsidian.vault` is not configured")?;
    let vault = expand_tilde(Path::new(vault)).into_owned();
    ensure!(
        vault.is_dir(),
        "Obsidian vault not found: {}",
        vault.display()
    );
    Ok((vault, config))
}

fn templates_dir(vault: &Path, config: &ObsidianConfig) -> Option<PathBuf> {
    (!config.templates_subdir.is_empty()).then(|| vault.join(&config.templates_subdir))
}

fn note_paths(vault: &Path, templates: Option<&Path>) -> Vec<PathBuf> {
    let mut walk = WalkBuilder::new(vault);
    walk.hidden(true)
        .parents(false)
        .ignore(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);

    let mut paths = Vec::new();
    for entry in walk.build().flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if templates.is_some_and(|templates| path.starts_with(templates)) {
            continue;
        }
        paths.push(path.to_path_buf());
    }
    paths.sort_by_key(|path| note_title(path).to_lowercase());
    paths
}

fn note_title(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn relative_path(vault: &Path, path: &Path) -> String {
    path.strip_prefix(vault)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn push_component<F>(cx: &mut CompositorContext, make: F)
where
    F: FnOnce(&mut helix_view::Editor, &mut crate::compositor::Compositor) + Send + 'static,
{
    let callback = async move {
        let call: job::Callback = job::Callback::EditorCompositor(Box::new(make));
        Ok(call)
    };
    cx.jobs.callback(callback);
}

fn push_note_picker(cx: &mut CompositorContext, items: Vec<NoteItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new("Results", |item: &NoteItem, _: &()| {
            format!("󰈔 {}", item.relative).into()
        })];
        let picker = Picker::new(columns, 0, items, (), |cx, item, action| {
            open_note(cx, &item.path, None, action);
        })
        .truncate_start(false)
        .with_title("Quick Switch")
        .with_help("<CR> open  <C-x> new note  <C-l> insert link")
        .with_extra_action(crate::ctrl!('x'), |cx, query, _| {
            let title = query.trim();
            if title.is_empty() {
                cx.editor.set_error("type a note title first");
                return false;
            }
            match create_and_open(cx, title) {
                Ok(()) => true,
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    false
                }
            }
        })
        .with_extra_action(crate::ctrl!('l'), |cx, _query, selection| {
            let Some(item) = selection else {
                cx.editor.set_error("no note selected");
                return false;
            };
            let title = note_title(&item.path);
            match insert_link(cx, &title) {
                Ok(()) => true,
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    false
                }
            }
        })
        .with_preview(|_editor, item| Some((item.path.as_path().into(), None)));
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_search_picker(cx: &mut CompositorContext, items: Vec<SearchItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new("Results", |item: &SearchItem, _: &()| {
            format!("󰈔 {}:{} {}", item.relative, item.line + 1, item.text).into()
        })];
        let picker = Picker::new(columns, 0, items, (), |cx, item, action| {
            open_note(cx, &item.path, Some(item.line), action);
        })
        .with_title("Results")
        .truncate_start(false)
        .with_help("<CR> open  <C-t> preview")
        .with_preview(|_editor, item| {
            Some((item.path.as_path().into(), Some((item.line, item.line))))
        });
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_backlink_picker(cx: &mut CompositorContext, items: Vec<BacklinkItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new(
            "Results",
            |item: &BacklinkItem, _: &()| {
                format!("󰈔 {}:{} {}", item.relative, item.line + 1, item.text).into()
            },
        )];
        let picker = Picker::new(columns, 0, items, (), |cx, item, action| {
            open_note(cx, &item.path, Some(item.line), action);
        })
        .with_title("Results")
        .truncate_start(false)
        .with_help("<CR> open  <C-t> preview")
        .with_preview(|_editor, item| {
            Some((item.path.as_path().into(), Some((item.line, item.line))))
        });
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn insert_link(cx: &mut CompositorContext, title: &str) -> anyhow::Result<()> {
    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    let doc = cx
        .editor
        .documents
        .get_mut(&doc_id)
        .context("no document in the current view")?;
    let selection = doc.selection(view_id).clone();
    let link = format!("[[{title}]]");
    let transaction = Transaction::insert(doc.text(), &selection, Tendril::from(link));
    doc.apply(&transaction, view_id);
    Ok(())
}

fn open_note(cx: &mut CompositorContext, path: &Path, line: Option<usize>, action: Action) {
    if let Err(err) = cx.editor.open(path, action) {
        cx.editor.set_error(err.to_string());
        return;
    }
    mark_note_recent(path);

    let Some(line) = line else {
        return;
    };

    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    if let Some(doc) = cx.editor.documents.get_mut(&doc_id) {
        let pos = Position::new(line, 0);
        let selection = Selection::point(pos_at_coords(doc.text().slice(..), pos, true));
        doc.set_selection(view_id, selection);
    }
}

fn create_and_open(cx: &mut CompositorContext, title: &str) -> anyhow::Result<()> {
    let path = create_note(cx, title)?;
    open_note(cx, &path, None, Action::Replace);
    Ok(())
}

fn current_line(cx: &CompositorContext) -> anyhow::Result<(PathBuf, String, usize)> {
    let view = cx.editor.tree.get(cx.editor.tree.focus);
    let doc = cx
        .editor
        .documents
        .get(&view.doc)
        .context("no document in the current view")?;
    let path = doc
        .path()
        .map(Path::to_path_buf)
        .context("the current buffer has no file path")?;
    let cursor_idx = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let cursor = coords_at_pos(doc.text().slice(..), cursor_idx);
    let line = doc.text().line(cursor.row).to_string();
    Ok((path, line, cursor.col))
}

fn run_search(cx: &mut CompositorContext, query: &str) -> anyhow::Result<()> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(());
    }
    let (vault, _) = vault_config(cx)?;

    let output = Command::new("rg")
        .args([
            "--line-number",
            "--no-heading",
            "--color",
            "never",
            "--smart-case",
            "--glob",
            "*.md",
            "--",
        ])
        .arg(query)
        .arg(&vault)
        .output()
        .context("failed to run ripgrep (`rg`)")?;

    if !output.status.success() && output.status.code() != Some(1) {
        bail!(
            "ripgrep failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(3, ':');
        let (Some(path), Some(line_number), Some(text)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(line_number) = line_number.parse::<usize>() else {
            continue;
        };
        let path = PathBuf::from(path);
        items.push(SearchItem {
            relative: relative_path(&vault, &path),
            path,
            line: line_number.saturating_sub(1),
            text: text.to_string(),
        });
    }

    if items.is_empty() {
        cx.editor.set_status(format!("no matches for `{query}`"));
        return Ok(());
    }

    push_search_picker(cx, items);
    Ok(())
}

fn link_at(line: &str, col: usize) -> Option<Link> {
    let col = col.min(line.chars().count());
    let byte_col = char_to_byte_idx(line, col);
    wiki_links(line)
        .into_iter()
        .chain(markdown_links(line))
        .find(|link| byte_col >= link.start && byte_col <= link.end)
}

fn wiki_links(line: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut offset = 0;
    while let Some(found) = line[offset..].find("[[") {
        let start = offset + found;
        let Some(close) = line[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + close + 2;
        let inner = &line[start + 2..start + 2 + close];
        let target = inner
            .split('|')
            .next()
            .unwrap_or_default()
            .split('#')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let start = if start > 0 && line.as_bytes()[start - 1] == b'!' {
            start - 1
        } else {
            start
        };
        links.push(Link {
            kind: LinkKind::Wiki,
            target,
            start,
            end,
        });
        offset = end;
    }
    links
}

fn markdown_links(line: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut offset = 0;
    while let Some(found) = line[offset..].find("](") {
        let close = offset + found;
        let Some(paren) = line[close + 2..].find(')') else {
            break;
        };
        let end = close + 2 + paren + 1;
        let start = line[..close].rfind('[').unwrap_or(close);
        let target = line[close + 2..end - 1]
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        links.push(Link {
            kind: LinkKind::Markdown,
            target,
            start,
            end,
        });
        offset = end;
    }
    links
}

fn resolve_link(vault: &Path, config: &ObsidianConfig, source: &Path, link: &Link) -> Vec<PathBuf> {
    let templates = templates_dir(vault, config);
    let notes = note_paths(vault, templates.as_deref());
    resolve_link_with_notes(vault, &notes, source, link)
}

fn resolve_link_with_notes(
    vault: &Path,
    notes: &[PathBuf],
    source: &Path,
    link: &Link,
) -> Vec<PathBuf> {
    match link.kind {
        LinkKind::Wiki => find_note_by_target(notes, vault, &link.target),
        LinkKind::Markdown => {
            let mut matches = Vec::new();
            if let Some(path) = resolve_markdown_path(source, vault, &link.target) {
                matches.push(path);
            }
            if matches.is_empty() {
                matches = find_note_by_target(notes, vault, &link.target);
            }
            matches
        }
    }
}

fn find_note_by_target(notes: &[PathBuf], vault: &Path, target: &str) -> Vec<PathBuf> {
    let target = target.trim();
    if target.is_empty() {
        return Vec::new();
    }
    let target = target.strip_suffix(".md").unwrap_or(target);
    notes
        .iter()
        .filter(|path| {
            note_title(path).eq_ignore_ascii_case(target)
                || relative_path(vault, path)
                    .trim_end_matches(".md")
                    .eq_ignore_ascii_case(target)
        })
        .cloned()
        .collect()
}

fn resolve_markdown_path(source: &Path, vault: &Path, target: &str) -> Option<PathBuf> {
    let target = target
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(|ch| ch == '<' || ch == '>');
    if target.is_empty() || target.starts_with("http://") || target.starts_with("https://") {
        return None;
    }

    let candidate = if let Some(absolute) = target.strip_prefix('/') {
        vault.join(absolute)
    } else {
        source.parent().unwrap_or(vault).join(target)
    };
    let candidate = if candidate.extension().is_none() {
        candidate.with_extension("md")
    } else {
        candidate
    };
    candidate.exists().then_some(candidate)
}

fn wiki_target_matches(target: &str, title: &str, vault: &Path, source: &Path) -> bool {
    let target = target.trim();
    target.eq_ignore_ascii_case(title)
        || relative_path(vault, source)
            .trim_end_matches(".md")
            .eq_ignore_ascii_case(target)
}

pub fn rename(cx: &mut CompositorContext, args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let title = args.join(" ");
    if title.trim().is_empty() {
        let current = current_note_path(cx)?;
        let title = note_title(&current);
        push_component(cx, move |editor, compositor| {
            let prompt = ui::Prompt::new(
                "rename note:".into(),
                None,
                ui::completers::none,
                |cx, input, event| {
                    if event != PromptEvent::Validate {
                        return;
                    }
                    if let Err(err) = rename_note(cx, input) {
                        cx.editor.set_error(err.to_string());
                    }
                },
            )
            .with_line(title, editor);
            compositor.push(Box::new(prompt));
        });
    } else {
        rename_note(cx, &title)?;
    }

    Ok(())
}

pub fn template(cx: &mut CompositorContext, args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let dir = templates_dir(&vault, &config).context("templates folder is not configured")?;
    ensure!(
        dir.is_dir(),
        "templates folder not found: {}",
        dir.display()
    );

    let mut items = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            items.push(TemplateItem {
                relative: relative_path(&vault, &path),
                path,
            });
        }
    }
    items.sort_by_key(|item| item.relative.to_lowercase());
    ensure!(!items.is_empty(), "no templates found in {}", dir.display());

    let name = args.join(" ");
    if name.trim().is_empty() {
        push_template_picker(cx, items);
    } else {
        let name = name.trim();
        let item = items
            .into_iter()
            .find(|item| note_title(&item.path).eq_ignore_ascii_case(name))
            .with_context(|| format!("template not found: {name}"))?;
        apply_template(cx, &item.path)?;
    }

    Ok(())
}

pub fn link(cx: &mut CompositorContext, _args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let items = note_items(cx)?;
    push_link_picker(cx, items);
    Ok(())
}

pub fn link_new(cx: &mut CompositorContext, _args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let texts = selection_texts(cx)?;
    let title = texts
        .first()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .context("select the text to use as the note title")?;
    let title = sanitize_title(title);
    let path = create_note(cx, &title)?;
    let title = note_title(&path);
    replace_selections(cx, &format!("[[{title}]]"))?;
    Ok(())
}

pub fn extract_note(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let texts = selection_texts(cx)?;
    let content = texts.join("\n");
    let first_line = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim();
    ensure!(!first_line.is_empty(), "select some text to extract");

    let title = sanitize_title(&first_line.chars().take(80).collect::<String>());
    let (vault, config) = vault_config(cx)?;
    let dir = notes_dir(&vault, &config);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{title}.md"));
    ensure!(!path.exists(), "a note named `{title}` already exists");
    fs::write(&path, content.trim_end())?;
    replace_selections(cx, &format!("[[{title}]]"))?;
    Ok(())
}

pub fn paste_image(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let types = Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .context("failed to run wl-paste")?;
    let types = String::from_utf8_lossy(&types.stdout);
    let (mime, extension) = if types.lines().any(|line| line.trim() == "image/png") {
        ("image/png", "png")
    } else if types
        .lines()
        .any(|line| matches!(line.trim(), "image/jpeg" | "image/jpg"))
    {
        ("image/jpeg", "jpg")
    } else {
        bail!("the clipboard does not contain a PNG or JPEG image");
    };

    let dir = notes_dir(&vault, &config).join(&config.attachments_subdir);
    fs::create_dir_all(&dir)?;
    let stamp = command_stdout("date", &["+%Y%m%d%H%M%S".to_string()]);
    let mut path = dir.join(format!("Pasted image {stamp}.{extension}"));
    let mut suffix = 2;
    while path.exists() {
        path = dir.join(format!("Pasted image {stamp}-{suffix}.{extension}"));
        suffix += 1;
    }
    let file = std::fs::File::create(&path)?;
    let status = match Command::new("wl-paste")
        .args(["--type", mime])
        .stdout(file)
        .status()
    {
        Ok(status) => status,
        Err(err) => {
            let _ = fs::remove_file(&path);
            return Err(err).context("failed to run wl-paste");
        }
    };
    if !status.success() {
        let _ = fs::remove_file(&path);
        bail!("wl-paste failed to write the image");
    }

    let link = current_note_path(cx)
        .ok()
        .and_then(|note| note.parent().map(Path::to_path_buf))
        .map(|base| relative_path_from(&base, &path))
        .unwrap_or_else(|| path.strip_prefix(&vault).unwrap_or(&path).to_path_buf());
    let link = link.display().to_string().replace('\\', "/");
    let link = link.replace(' ', "%20");
    insert_text(cx, &format!("![]({link})"))
}

pub fn toggle_checkbox(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    let doc = cx
        .editor
        .documents
        .get_mut(&doc_id)
        .context("no document in the current view")?;
    let text = doc.text().slice(..);

    let mut changes = Vec::new();
    for range in doc.selection(view_id).iter() {
        let line_idx = range.cursor_line(text);
        let line = text.line(line_idx).to_string();
        let Some(pos) = checkbox_char_position(&line) else {
            continue;
        };
        let marker = if matches!(line.chars().nth(pos + 1), Some('x' | 'X')) {
            " "
        } else {
            "x"
        };
        let line_start = text.line_to_char(line_idx);
        changes.push((
            line_start + pos + 1,
            line_start + pos + 2,
            Some(Tendril::from(marker)),
        ));
    }

    if changes.is_empty() {
        cx.editor.set_status("no checkbox on the current line");
        return Ok(());
    }

    let transaction = Transaction::change(doc.text(), changes.into_iter());
    doc.apply(&transaction, view_id);
    Ok(())
}

pub fn toc(cx: &mut CompositorContext, _args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let text = current_doc_text(cx)?;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let heading = trimmed.trim_start_matches('#');
        let level = trimmed.len() - heading.len();
        let heading = heading.trim();
        if level == 0 || heading.is_empty() {
            continue;
        }
        let indent = "  ".repeat(level.saturating_sub(1));
        lines.push(format!("{indent}- [[#{heading}]]"));
    }

    ensure!(!lines.is_empty(), "the current note has no headings");
    insert_text(cx, &format!("{}\n", lines.join("\n")))
}

pub fn links(cx: &mut CompositorContext, _args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let source = current_note_path(cx)?;
    let text = current_doc_text(cx)?;
    // Resolve every link against one vault scan instead of walking the vault
    // once per link.
    let templates = templates_dir(&vault, &config);
    let notes = note_paths(&vault, templates.as_deref());
    let mut items = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        for link in wiki_links(line).into_iter().chain(markdown_links(line)) {
            let path = resolve_link_with_notes(&vault, &notes, &source, &link)
                .into_iter()
                .next();
            items.push(LinkItem {
                path,
                target: link.target,
                line: line_idx,
            });
        }
    }

    if items.is_empty() {
        cx.editor.set_status("the current note has no links");
        return Ok(());
    }

    push_link_list_picker(cx, items);
    Ok(())
}

pub fn tags(cx: &mut CompositorContext, _args: Args, event: PromptEvent) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (vault, config) = vault_config(cx)?;
    let templates = templates_dir(&vault, &config);
    let mut tags = BTreeSet::new();
    for path in note_paths(&vault, templates.as_deref()) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for word in text.split_whitespace() {
            let Some(tag) = word.strip_prefix('#') else {
                continue;
            };
            let tag = tag.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '/' && ch != '_');
            if tag.chars().next().is_some_and(char::is_alphanumeric) {
                tags.insert(format!("#{tag}"));
            }
        }
    }

    ensure!(!tags.is_empty(), "no tags found in the vault");
    push_tag_picker(cx, tags.into_iter().collect());
    Ok(())
}

pub fn smart_action(
    cx: &mut CompositorContext,
    _args: Args,
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    let (_, line, col) = current_line(cx)?;
    if link_at(&line, col).is_some() {
        follow_link(cx, Args::default(), PromptEvent::Validate)
    } else {
        toggle_checkbox(cx, Args::default(), PromptEvent::Validate)
    }
}

fn rename_note(cx: &mut CompositorContext, new_title: &str) -> anyhow::Result<()> {
    let new_title = new_title.trim();
    ensure!(!new_title.is_empty(), "note title cannot be empty");
    ensure!(
        !new_title.contains(['/', '\\', '\0']),
        "note title contains characters that are not allowed in a file name"
    );

    let (vault, config) = vault_config(cx)?;
    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    let (old_path, modified, contents) = {
        let doc = cx
            .editor
            .documents
            .get(&doc_id)
            .context("no document in the current view")?;
        let path = doc
            .path()
            .map(Path::to_path_buf)
            .context("the current buffer has no file path")?;
        (path, doc.is_modified(), doc.text().to_string())
    };
    ensure!(
        old_path.starts_with(&vault),
        "the current note is not inside the vault"
    );

    let old_title = note_title(&old_path);
    let new_path = old_path.with_file_name(format!("{new_title}.md"));
    ensure!(
        new_path != old_path,
        "the new title is the same as the current title"
    );
    ensure!(
        !new_path.exists(),
        "a note named `{new_title}` already exists"
    );

    let old_relative = relative_path(&vault, &old_path);
    let new_relative = relative_path(&vault, &new_path);
    if modified {
        fs::write(&old_path, contents)?;
    }
    // Rename before touching links: if the rename fails (permissions, a
    // concurrent file, etc.) no other note has been rewritten to a title that
    // does not exist on disk.
    fs::rename(&old_path, &new_path)?;
    cx.editor.set_doc_path(doc_id, &new_path);

    let templates = templates_dir(&vault, &config);
    let mut backups: Vec<(PathBuf, String)> = Vec::new();
    let rewrite_result = (|| -> anyhow::Result<()> {
        for path in note_paths(&vault, templates.as_deref()) {
            if path == new_path {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let updated = rewrite_links(
                &text,
                &old_title,
                &old_relative,
                new_title,
                &new_relative,
                &vault,
                &path,
            );
            if updated != text {
                backups.push((path.clone(), text));
                fs::write(&path, updated)
                    .with_context(|| format!("failed to rewrite links in {}", path.display()))?;
            }
        }
        Ok(())
    })();

    if let Err(err) = rewrite_result {
        // Best-effort transaction: restore every file we already rewrote,
        // then move the note back so links never point at a renamed file
        // that only exists for some notes.
        let mut rollback_errors = Vec::new();
        for (path, original) in backups.iter().rev() {
            if let Err(rollback) = fs::write(path, original) {
                rollback_errors.push(format!("{}: {rollback}", path.display()));
            }
        }
        match fs::rename(&new_path, &old_path) {
            Ok(()) => cx.editor.set_doc_path(doc_id, &old_path),
            Err(rollback) => rollback_errors.push(format!(
                "could not move {} back to {}: {rollback}",
                new_path.display(),
                old_path.display()
            )),
        }
        if rollback_errors.is_empty() {
            return Err(err);
        }
        bail!(
            "{err}; rollback also failed: {}",
            rollback_errors.join("; ")
        );
    }

    cx.editor.set_status(format!("renamed note to {new_title}"));
    Ok(())
}

fn rewrite_links(
    text: &str,
    old_title: &str,
    old_relative: &str,
    new_title: &str,
    new_relative: &str,
    vault: &Path,
    source: &Path,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut offset = 0;
    while let Some(found) = text[offset..].find("[[") {
        let start = offset + found;
        let Some(close) = text[start + 2..].find("]]") else {
            break;
        };
        let inner_start = start + 2;
        let inner_end = inner_start + close;
        out.push_str(&text[offset..start]);
        let inner = &text[inner_start..inner_end];
        let (target, suffix) = split_target(inner);
        if target_matches(target, old_title, old_relative) {
            let replacement = if target.contains(['/', '\\']) {
                new_relative.trim_end_matches(".md").to_string()
            } else {
                new_title.to_string()
            };
            out.push_str(&format!("[[{replacement}{suffix}]]"));
        } else {
            out.push_str(&format!("[[{inner}]]"));
        }
        offset = inner_end + 2;
    }
    out.push_str(&text[offset..]);

    rewrite_markdown_links(out, old_title, old_relative, new_title, vault, source)
}

fn rewrite_markdown_links(
    mut text: String,
    old_title: &str,
    old_relative: &str,
    new_title: &str,
    vault: &Path,
    source: &Path,
) -> String {
    let old_path = vault.join(old_relative);
    let old_canonical = fs::canonicalize(&old_path).ok();
    let mut offset = 0;
    while let Some(found) = text[offset..].find("](") {
        let target_start = offset + found + 2;
        let Some(close) = text[target_start..].find(')') else {
            break;
        };
        let target_end = target_start + close;
        let target = text[target_start..target_end].to_string();
        let (path_part, suffix) = match target.find('#') {
            Some(index) => (&target[..index], &target[index..]),
            None => (target.as_str(), ""),
        };
        let cleaned = path_part.trim().trim_matches(|ch| ch == '<' || ch == '>');
        let Some(cleaned) = Some(cleaned).filter(|cleaned| !cleaned.is_empty()) else {
            offset = target_end + 1;
            continue;
        };
        let resolved = resolve_markdown_path(source, vault, cleaned);
        let matched = resolved
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|path| Some(&path) == old_canonical.as_ref())
            || cleaned
                .trim_end_matches(".md")
                .eq_ignore_ascii_case(old_title);
        if matched {
            let replacement = if path_part.ends_with(".md") {
                format!("{new_title}.md{suffix}")
            } else {
                format!("{new_title}{suffix}")
            };
            text.replace_range(target_start..target_end, &replacement);
            offset = target_start + replacement.len();
        } else {
            offset = target_end + 1;
        }
    }
    text
}

fn split_target(inner: &str) -> (&str, &str) {
    let index = inner.find(['|', '#']).unwrap_or(inner.len());
    (&inner[..index], &inner[index..])
}

fn target_matches(target: &str, title: &str, relative: &str) -> bool {
    let target = target.trim().trim_end_matches(".md");
    target.eq_ignore_ascii_case(title)
        || target.eq_ignore_ascii_case(relative.trim_end_matches(".md"))
}

fn note_items(cx: &CompositorContext) -> anyhow::Result<Vec<NoteItem>> {
    let (vault, config) = vault_config(cx)?;
    let templates = templates_dir(&vault, &config);
    let paths = note_paths(&vault, templates.as_deref());
    ensure!(
        !paths.is_empty(),
        "no markdown notes found in {}",
        vault.display()
    );
    Ok(paths
        .into_iter()
        .map(|path| NoteItem {
            relative: relative_path(&vault, &path),
            path,
        })
        .collect())
}

pub fn complete_link(cx: &mut crate::commands::Context) -> bool {
    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    let (start, mut end, needs_second_bracket, cursor) = {
        let Some(doc) = cx.editor.documents.get(&doc_id) else {
            return false;
        };
        let Some(path) = doc.path() else {
            return false;
        };
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            return false;
        }
        let selection = doc.selection(view_id);
        if selection.len() != 1 {
            return false;
        }
        let text = doc.text().slice(..);
        let cursor = selection.primary().cursor(text);
        let Some((start, end)) = wiki_completion_range(&text, cursor) else {
            return false;
        };
        // The second `[` triggered this command before insert mode could
        // handle it. Insert it now so cancelling the picker leaves `[[`
        // instead of a single swallowed bracket.
        (start, end, end == cursor, cursor)
    };

    let Ok((vault, config)) = vault_config_for(cx.editor) else {
        return false;
    };
    let templates = templates_dir(&vault, &config);

    if needs_second_bracket {
        if let Some(doc) = cx.editor.documents.get_mut(&doc_id) {
            let transaction =
                Transaction::insert(doc.text(), &Selection::point(cursor), Tendril::from("["));
            doc.apply(&transaction, view_id);
            end += 1;
        } else {
            return false;
        }
    }

    let version = match cx.editor.documents.get(&doc_id) {
        Some(doc) => doc.version(),
        None => return false,
    };

    cx.jobs.callback(async move {
        let paths = note_paths(&vault, templates.as_deref());
        ensure!(
            !paths.is_empty(),
            "no markdown notes found in {}",
            vault.display()
        );
        let items: Vec<NoteItem> = paths
            .into_iter()
            .map(|path| NoteItem {
                relative: relative_path(&vault, &path),
                path,
            })
            .collect();
        let call = job::Callback::EditorCompositor(Box::new(
            move |editor: &mut helix_view::Editor,
                  compositor: &mut crate::compositor::Compositor| {
                // The document changed while the vault scan was running;
                // leave the typed `[[` alone rather than editing stale text.
                if editor
                    .documents
                    .get(&doc_id)
                    .is_none_or(|doc| doc.version() != version)
                {
                    return;
                }
                push_wiki_picker(compositor, items, start, end);
            },
        ));
        Ok(call)
    });
    true
}

fn wiki_completion_range(text: &helix_core::RopeSlice, cursor: usize) -> Option<(usize, usize)> {
    if cursor == 0 || text.char(cursor - 1) != '[' {
        return None;
    }
    let end = if cursor < text.len_chars() && text.char(cursor) == ']' {
        cursor + 1
    } else {
        cursor
    };
    Some((cursor - 1, end))
}

fn push_wiki_picker(
    compositor: &mut crate::compositor::Compositor,
    items: Vec<NoteItem>,
    start: usize,
    end: usize,
) {
    let columns = [PickerColumn::new("Results", |item: &NoteItem, _: &()| {
        format!("󰈔 {}", item.relative).into()
    })];
    let picker = Picker::new(columns, 0, items, (), move |cx, item, _action| {
        let title = note_title(&item.path);
        let view_id = cx.editor.tree.focus;
        let doc_id = cx.editor.tree.get(view_id).doc;
        if let Some(doc) = cx.editor.documents.get_mut(&doc_id) {
            let transaction = Transaction::change(
                doc.text(),
                std::iter::once((start, end, Some(Tendril::from(format!("[[{title}]]"))))),
            );
            doc.apply(&transaction, view_id);
        }
    })
    .with_title("Insert Link")
    .with_help("<CR> insert link  <C-t> preview")
    .truncate_start(false)
    .with_preview(|_editor, item| Some((item.path.as_path().into(), None)));
    compositor.push(Box::new(overlaid(picker)));
}

fn push_link_picker(cx: &mut CompositorContext, items: Vec<NoteItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new("Results", |item: &NoteItem, _: &()| {
            format!("󰈔 {}", item.relative).into()
        })];
        let picker = Picker::new(columns, 0, items, (), |cx, item, _action| {
            let title = note_title(&item.path);
            if let Err(err) = replace_selections(cx, &format!("[[{title}]]")) {
                cx.editor.set_error(err.to_string());
            }
        })
        .with_title("Insert Link")
        .with_help("<CR> insert link  <C-t> preview")
        .truncate_start(false)
        .with_preview(|_editor, item| Some((item.path.as_path().into(), None)));
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_link_list_picker(cx: &mut CompositorContext, items: Vec<LinkItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new("Results", |item: &LinkItem, _: &()| {
            if item.path.is_some() {
                format!("󰌷 {} (line {})", item.target, item.line + 1).into()
            } else {
                format!("󰌷 {} (missing)", item.target).into()
            }
        })];
        let picker = Picker::new(columns, 0, items, (), |cx, item, action| {
            if let Some(path) = &item.path {
                open_note(cx, path, Some(item.line), action);
            } else if let Err(err) = create_and_open(cx, &item.target) {
                cx.editor.set_error(err.to_string());
            }
        })
        .with_title("Links")
        .with_help("<CR> follow  <C-t> preview")
        .truncate_start(false)
        .with_preview(|_editor, item| {
            item.path
                .as_ref()
                .map(|path| (path.as_path().into(), Some((item.line, item.line))))
        });
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_template_picker(cx: &mut CompositorContext, items: Vec<TemplateItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new(
            "Results",
            |item: &TemplateItem, _: &()| format!("󰈔 {}", item.relative).into(),
        )];
        let picker = Picker::new(columns, 0, items, (), |cx, item, _action| {
            if let Err(err) = apply_template(cx, &item.path) {
                cx.editor.set_error(err.to_string());
            }
        })
        .with_title("Templates")
        .with_help("<CR> apply  <C-t> preview")
        .truncate_start(false)
        .with_preview(|_editor, item| Some((item.path.as_path().into(), None)));
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_tag_picker(cx: &mut CompositorContext, tags: Vec<String>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [PickerColumn::new("Results", |tag: &String, _: &()| {
            tag.clone().into()
        })];
        let picker = Picker::new(columns, 0, tags, (), |cx, tag, _action| {
            if let Err(err) = insert_text(cx, tag) {
                cx.editor.set_error(err.to_string());
            }
        })
        .with_title("Tags")
        .with_help("<CR> insert tag  <C-t> preview");
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn apply_template(cx: &mut CompositorContext, path: &Path) -> anyhow::Result<()> {
    let (_, config) = vault_config(cx)?;
    let template = fs::read_to_string(path)?;
    let title = current_note_path(cx)
        .map(|path| note_title(&path))
        .unwrap_or_default();
    let rendered = render_template(&template, &title, &config);
    insert_text(cx, &rendered)
}

fn render_template(template: &str, title: &str, config: &ObsidianConfig) -> String {
    let date = command_stdout("date", &[format!("+{}", config.date_format)]);
    let time = command_stdout("date", &[format!("+{}", config.time_format)]);
    template
        .replace("{{title}}", title)
        .replace("{{date}}", &date)
        .replace("{{time}}", &time)
}

fn command_stdout(program: &str, args: &[String]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

fn current_note_path(cx: &CompositorContext) -> anyhow::Result<PathBuf> {
    let view = cx.editor.tree.get(cx.editor.tree.focus);
    let doc = cx
        .editor
        .documents
        .get(&view.doc)
        .context("no document in the current view")?;
    doc.path()
        .map(Path::to_path_buf)
        .context("the current buffer has no file path")
}

fn current_doc_text(cx: &CompositorContext) -> anyhow::Result<String> {
    let view = cx.editor.tree.get(cx.editor.tree.focus);
    let doc = cx
        .editor
        .documents
        .get(&view.doc)
        .context("no document in the current view")?;
    Ok(doc.text().to_string())
}

fn selection_texts(cx: &CompositorContext) -> anyhow::Result<Vec<String>> {
    let view = cx.editor.tree.get(cx.editor.tree.focus);
    let doc = cx
        .editor
        .documents
        .get(&view.doc)
        .context("no document in the current view")?;
    let text = doc.text().slice(..);
    Ok(doc
        .selection(view.id)
        .slices(text)
        .map(|slice| slice.to_string())
        .collect())
}

fn replace_selections(cx: &mut CompositorContext, replacement: &str) -> anyhow::Result<()> {
    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    let doc = cx
        .editor
        .documents
        .get_mut(&doc_id)
        .context("no document in the current view")?;
    let selection = doc.selection(view_id).clone();
    let transaction = Transaction::change_by_selection(doc.text(), &selection, |range| {
        (range.from(), range.to(), Some(Tendril::from(replacement)))
    });
    doc.apply(&transaction, view_id);
    Ok(())
}

fn insert_text(cx: &mut CompositorContext, text: &str) -> anyhow::Result<()> {
    let view_id = cx.editor.tree.focus;
    let doc_id = cx.editor.tree.get(view_id).doc;
    let doc = cx
        .editor
        .documents
        .get_mut(&doc_id)
        .context("no document in the current view")?;
    let selection = doc.selection(view_id).clone();
    let transaction = Transaction::insert(doc.text(), &selection, Tendril::from(text));
    doc.apply(&transaction, view_id);
    Ok(())
}

fn notes_dir(vault: &Path, config: &ObsidianConfig) -> PathBuf {
    if config.notes_subdir.is_empty() {
        vault.to_path_buf()
    } else {
        vault.join(&config.notes_subdir)
    }
}

fn create_note(cx: &mut CompositorContext, title: &str) -> anyhow::Result<PathBuf> {
    let title = title.trim();
    ensure!(!title.is_empty(), "note title cannot be empty");
    ensure!(
        !title.contains(['/', '\\', '\0']),
        "note title contains characters that are not allowed in a file name"
    );

    let (vault, config) = vault_config(cx)?;
    let dir = notes_dir(&vault, &config);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{title}.md"));
    if !path.exists() {
        fs::write(&path, "")?;
    }
    Ok(path)
}

fn sanitize_title(title: &str) -> String {
    title
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | '\0') {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn checkbox_position(line: &str) -> Option<usize> {
    line.as_bytes().windows(3).position(|window| {
        window[0] == b'[' && window[2] == b']' && matches!(window[1], b' ' | b'x' | b'X')
    })
}

/// Char offset of the checkbox marker, converting the byte offset returned by
/// `checkbox_position` so non-ASCII prefixes don't shift the edit.
fn checkbox_char_position(line: &str) -> Option<usize> {
    checkbox_position(line).map(|pos| line[..pos].chars().count())
}

fn relative_path_from(base: &Path, target: &Path) -> PathBuf {
    let base: Vec<Component> = base.components().collect();
    let target: Vec<Component> = target.components().collect();
    let common = base
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut result = PathBuf::new();
    for _ in common..base.len() {
        result.push("..");
    }
    for component in &target[common..] {
        result.push(component.as_os_str());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_wiki_link_under_cursor() {
        let line = "see [[My Note|alias]] now";
        let cursor = line.find("My Note").unwrap();
        let link = link_at(line, line[..cursor].chars().count()).unwrap();
        assert_eq!(link.kind, LinkKind::Wiki);
        assert_eq!(link.target, "My Note");
    }

    #[test]
    fn strips_wiki_heading_and_alias() {
        let links = wiki_links("[[Note#Heading|label]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Note");
    }

    #[test]
    fn finds_markdown_link_under_cursor() {
        let line = "![alt](assets/image.png)";
        let cursor = line.find("assets").unwrap();
        let link = link_at(line, line[..cursor].chars().count()).unwrap();
        assert_eq!(link.kind, LinkKind::Markdown);
        assert_eq!(link.target, "assets/image.png");
    }

    #[test]
    fn note_title_is_the_file_stem() {
        assert_eq!(note_title(Path::new("/vault/6 - Main Notes/Foo.md")), "Foo");
    }

    #[test]
    fn rewrites_wiki_links_when_renaming() {
        let text = "[[Old]] [[Old|alias]] [[Old#Heading]] [[Other]] ![[Old]]";
        let rewritten = rewrite_links(
            text,
            "Old",
            "6 - Main Notes/Old.md",
            "New",
            "6 - Main Notes/New.md",
            Path::new("/vault"),
            Path::new("/vault/6 - Main Notes/Source.md"),
        );
        assert_eq!(
            rewritten,
            "[[New]] [[New|alias]] [[New#Heading]] [[Other]] ![[New]]"
        );
    }

    #[test]
    fn split_target_keeps_heading_and_alias() {
        assert_eq!(
            split_target("Note#Heading|alias"),
            ("Note", "#Heading|alias")
        );
        assert_eq!(split_target("Note"), ("Note", ""));
    }

    #[test]
    fn finds_checkbox_markers() {
        assert_eq!(checkbox_position("- [ ] task"), Some(2));
        assert_eq!(checkbox_position("- [x] done"), Some(2));
        assert_eq!(checkbox_position("- [X] done"), Some(2));
        assert_eq!(checkbox_position("- plain"), None);
    }

    #[test]
    fn checkbox_marker_uses_char_offsets() {
        // `é` is two bytes, so the byte offset and char offset differ.
        assert_eq!(checkbox_char_position("é - [ ] task"), Some(4));
        assert_eq!(checkbox_char_position("- [ ] task"), Some(2));
    }

    #[test]
    fn builds_relative_attachment_paths() {
        assert_eq!(
            relative_path_from(
                Path::new("/vault/6 - Main Notes"),
                Path::new("/vault/6 - Main Notes/assets/img.png"),
            ),
            Path::new("assets/img.png")
        );
        assert_eq!(
            relative_path_from(
                Path::new("/vault/1 - Rough Notes"),
                Path::new("/vault/6 - Main Notes/assets/img.png"),
            ),
            Path::new("../6 - Main Notes/assets/img.png")
        );
    }

    #[test]
    fn quick_switch_orders_by_recency_then_name() {
        use std::time::Duration;

        let item = |name: &str| NoteItem {
            path: PathBuf::from(format!("/tmp/{name}.md")),
            relative: name.to_string(),
        };
        let now = Instant::now();
        let mut recency = HashMap::new();
        recency.insert(PathBuf::from("/tmp/recent.md"), now);
        recency.insert(
            PathBuf::from("/tmp/older.md"),
            now.checked_sub(Duration::from_secs(5)).unwrap(),
        );

        let items = vec![item("alpha"), item("recent"), item("delta"), item("older")];
        let ordered = order_quick_switch_items(items, &recency);
        let names: Vec<&str> = ordered.iter().map(|item| item.relative.as_str()).collect();
        assert_eq!(names, vec!["recent", "older", "alpha", "delta"]);
    }

    #[test]
    fn detects_wiki_completion_pair() {
        use helix_core::Rope;

        let text = Rope::from("[]");
        assert_eq!(wiki_completion_range(&text.slice(..), 1), Some((0, 2)));

        let text = Rope::from("[");
        assert_eq!(wiki_completion_range(&text.slice(..), 1), Some((0, 1)));

        let text = Rope::from("a");
        assert_eq!(wiki_completion_range(&text.slice(..), 1), None);
    }
}
