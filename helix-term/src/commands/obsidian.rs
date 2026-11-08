use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, ensure, Context as _};
use helix_core::{
    command_line::Args, coords_at_pos, pos_at_coords, str_utils::char_to_byte_idx, Position,
    Selection,
};
use helix_stdx::path::expand_tilde;
use helix_view::editor::{Action, ObsidianConfig};
use ignore::WalkBuilder;

use crate::compositor::Context as CompositorContext;
use crate::job;
use crate::ui::{self, overlay::overlaid, Picker, PickerColumn, PromptEvent};

struct NoteItem {
    path: PathBuf,
    title: String,
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

    let items = paths
        .into_iter()
        .map(|path| NoteItem {
            title: note_title(&path),
            relative: relative_path(&vault, &path),
            path,
        })
        .collect();

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
                    title: note_title(&path),
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
    let config = cx.editor.config().obsidian.clone();
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
        let columns = [
            PickerColumn::new("note", |item: &NoteItem, _: &()| item.title.clone().into()),
            PickerColumn::new("path", |item: &NoteItem, _: &()| {
                item.relative.clone().into()
            }),
        ];
        let picker = Picker::new(columns, 0, items, (), |cx, item, action| {
            open_note(cx, &item.path, None, action);
        })
        .with_preview(|_editor, item| Some((item.path.as_path().into(), None)));
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_search_picker(cx: &mut CompositorContext, items: Vec<SearchItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [
            PickerColumn::new("path", |item: &SearchItem, _: &()| {
                format!("{}:{}", item.relative, item.line + 1).into()
            }),
            PickerColumn::new("text", |item: &SearchItem, _: &()| item.text.clone().into()),
        ];
        let picker = Picker::new(columns, 1, items, (), |cx, item, action| {
            open_note(cx, &item.path, Some(item.line), action);
        })
        .with_preview(|_editor, item| {
            Some((item.path.as_path().into(), Some((item.line, item.line))))
        });
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn push_backlink_picker(cx: &mut CompositorContext, items: Vec<BacklinkItem>) {
    push_component(cx, move |_editor, compositor| {
        let columns = [
            PickerColumn::new("path", |item: &BacklinkItem, _: &()| {
                format!("{}:{}", item.relative, item.line + 1).into()
            }),
            PickerColumn::new("text", |item: &BacklinkItem, _: &()| {
                item.text.clone().into()
            }),
        ];
        let picker = Picker::new(columns, 1, items, (), |cx, item, action| {
            open_note(cx, &item.path, Some(item.line), action);
        })
        .with_preview(|_editor, item| {
            Some((item.path.as_path().into(), Some((item.line, item.line))))
        });
        compositor.push(Box::new(overlaid(picker)));
    });
}

fn open_note(cx: &mut CompositorContext, path: &Path, line: Option<usize>, action: Action) {
    if let Err(err) = cx.editor.open(path, action) {
        cx.editor.set_error(err.to_string());
        return;
    }

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
    let title = title.trim();
    ensure!(!title.is_empty(), "note title cannot be empty");
    ensure!(
        !title.contains(['/', '\\', '\0']),
        "note title contains characters that are not allowed in a file name"
    );

    let (vault, config) = vault_config(cx)?;
    let dir = if config.notes_subdir.is_empty() {
        vault
    } else {
        vault.join(&config.notes_subdir)
    };
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{title}.md"));
    if !path.exists() {
        fs::write(&path, "")?;
    }

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

    match link.kind {
        LinkKind::Wiki => find_note_by_target(&notes, vault, &link.target),
        LinkKind::Markdown => {
            let mut matches = Vec::new();
            if let Some(path) = resolve_markdown_path(source, vault, &link.target) {
                matches.push(path);
            }
            if matches.is_empty() {
                matches = find_note_by_target(&notes, vault, &link.target);
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
}
