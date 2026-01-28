mod completion;
mod document;
pub(crate) mod editor;
mod info;
pub mod lsp;
mod markdown;
pub mod menu;
pub mod overlay;
pub mod picker;
pub mod popup;
pub mod prompt;
mod select;
mod spinner;
mod statusline;
mod text;
mod text_decorations;

use crate::compositor::Compositor;
use crate::filter_picker_entry;
use crate::job::{self, Callback};
pub use completion::Completion;
pub use editor::EditorView;
use helix_stdx::rope;
use helix_view::theme::Style;
pub use markdown::Markdown;
pub use menu::Menu;
pub use picker::{Column as PickerColumn, FileLocation, Picker};
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use select::Select;
pub use spinner::{ProgressSpinners, Spinner};
pub use text::Text;

use helix_view::text::{Span, Spans};
use helix_view::Editor;

use std::path::Path;
use std::{error::Error, path::PathBuf};

pub fn prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    callback_fn: impl FnMut(&mut crate::compositor::Context, &str, PromptEvent) + 'static,
) {
    let mut prompt = Prompt::new(prompt, history_register, completion_fn, callback_fn);
    // Calculate the initial completion
    prompt.recalculate_completion(cx.editor);
    cx.push_layer(Box::new(prompt));
}

pub fn prompt_with_input(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    input: String,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    callback_fn: impl FnMut(&mut crate::compositor::Context, &str, PromptEvent) + 'static,
) {
    let prompt = Prompt::new(prompt, history_register, completion_fn, callback_fn)
        .with_line(input, cx.editor);
    cx.push_layer(Box::new(prompt));
}

pub fn regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    fun: impl Fn(&mut crate::compositor::Context, rope::Regex, PromptEvent) + 'static,
) {
    raw_regex_prompt(
        cx,
        prompt,
        history_register,
        completion_fn,
        move |cx, regex, _, event| fun(cx, regex, event),
    );
}
pub fn raw_regex_prompt(
    cx: &mut crate::commands::Context,
    prompt: std::borrow::Cow<'static, str>,
    history_register: Option<char>,
    completion_fn: impl FnMut(&Editor, &str) -> Vec<prompt::Completion> + 'static,
    fun: impl Fn(&mut crate::compositor::Context, rope::Regex, &str, PromptEvent) + 'static,
) {
    let (view, doc) = current!(cx.editor);
    let doc_id = view.doc;
    let snapshot = doc.selection(view.id).clone();
    let offset_snapshot = doc.view_offset(view.id);
    let config = cx.editor.config();

    let mut prompt = Prompt::new(
        prompt,
        history_register,
        completion_fn,
        move |cx: &mut crate::compositor::Context, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    let (view, doc) = current!(cx.editor);
                    doc.set_selection(view.id, snapshot.clone());
                    doc.set_view_offset(view.id, offset_snapshot);
                }
                PromptEvent::Update | PromptEvent::Validate => {
                    // skip empty input
                    if input.is_empty() {
                        return;
                    }

                    let case_insensitive = if config.search.smart_case {
                        !input.chars().any(char::is_uppercase)
                    } else {
                        false
                    };

                    match rope::RegexBuilder::new()
                        .syntax(
                            rope::Config::new()
                                .case_insensitive(case_insensitive)
                                .multi_line(true),
                        )
                        .build(input)
                    {
                        Ok(regex) => {
                            let (view, doc) = current!(cx.editor);

                            // revert state to what it was before the last update
                            doc.set_selection(view.id, snapshot.clone());

                            if event == PromptEvent::Validate {
                                // Equivalent to push_jump to store selection just before jump
                                view.jumps.push((doc_id, snapshot.clone()));
                            }

                            fun(cx, regex, input, event);

                            let (view, doc) = current!(cx.editor);
                            view.ensure_cursor_in_view(doc, config.scrolloff);
                        }
                        Err(err) => {
                            let (view, doc) = current!(cx.editor);
                            doc.set_selection(view.id, snapshot.clone());
                            doc.set_view_offset(view.id, offset_snapshot);

                            if event == PromptEvent::Validate {
                                let callback = async move {
                                    let call: job::Callback = Callback::EditorCompositor(Box::new(
                                        move |_editor: &mut Editor, compositor: &mut Compositor| {
                                            let contents = Text::new(format!("{}", err));
                                            let size = compositor.size();
                                            let popup = Popup::new("invalid-regex", contents)
                                                .position(Some(helix_core::Position::new(
                                                    size.height as usize - 2, // 2 = statusline + commandline
                                                    0,
                                                )))
                                                .auto_close(true);
                                            compositor.replace_or_push("invalid-regex", popup);
                                        },
                                    ));
                                    Ok(call)
                                };

                                cx.jobs.callback(callback);
                            }
                        }
                    }
                }
            }
        },
    )
    .with_language("regex", std::sync::Arc::clone(&cx.editor.syn_loader));
    // Calculate initial completion
    prompt.recalculate_completion(cx.editor);
    // prompt
    cx.push_layer(Box::new(prompt));
}

/// We want to exclude files that the editor can't handle yet
fn get_excluded_types() -> ignore::types::Types {
    use ignore::types::TypesBuilder;
    let mut type_builder = TypesBuilder::new();
    type_builder
        .add(
            "compressed",
            "*.{zip,gz,bz2,zst,lzo,sz,tgz,tbz2,lz,lz4,lzma,lzo,z,Z,xz,7z,rar,cab}",
        )
        .expect("Invalid type definition");
    type_builder.negate("all");
    type_builder
        .build()
        .expect("failed to build excluded_types")
}

#[derive(Debug)]
pub struct FilePickerData {
    root: PathBuf,
    directory_style: Style,
}
type FilePicker = Picker<PathBuf, FilePickerData>;

pub fn file_picker(editor: &Editor, root: PathBuf) -> FilePicker {
    use ignore::WalkBuilder;
    use std::time::Instant;

    let config = editor.config();
    let data = FilePickerData {
        root: root.clone(),
        directory_style: editor.theme.get("ui.text.directory"),
    };

    let now = Instant::now();

    let dedup_symlinks = config.file_picker.deduplicate_links;
    let absolute_root = root.canonicalize().unwrap_or_else(|_| root.clone());

    let mut walk_builder = WalkBuilder::new(&root);

    let mut files = walk_builder
        .hidden(config.file_picker.hidden)
        .parents(config.file_picker.parents)
        .ignore(config.file_picker.ignore)
        .follow_links(config.file_picker.follow_symlinks)
        .git_ignore(config.file_picker.git_ignore)
        .git_global(config.file_picker.git_global)
        .git_exclude(config.file_picker.git_exclude)
        .sort_by_file_name(|name1, name2| name1.cmp(name2))
        .max_depth(config.file_picker.max_depth)
        .filter_entry(move |entry| filter_picker_entry(entry, &absolute_root, dedup_symlinks))
        .add_custom_ignore_filename(helix_loader::config_dir().join("ignore"))
        .add_custom_ignore_filename(".helix/ignore")
        .types(get_excluded_types())
        .build()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type()?.is_file() {
                return None;
            }
            Some(entry.into_path())
        });
    log::debug!("file_picker init {:?}", Instant::now().duration_since(now));

    let columns = [PickerColumn::new(
        "path",
        |item: &PathBuf, data: &FilePickerData| {
            let path = item.strip_prefix(&data.root).unwrap_or(item);
            let mut spans = Vec::with_capacity(3);
            if let Some(dirs) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                spans.extend([
                    Span::styled(dirs.to_string_lossy(), data.directory_style),
                    Span::styled(std::path::MAIN_SEPARATOR_STR, data.directory_style),
                ]);
            }
            let filename = path
                .file_name()
                .expect("normalized paths can't end in `..`")
                .to_string_lossy();
            spans.push(Span::raw(filename));
            Spans::from(spans).into()
        },
    )];
    let picker = Picker::new(columns, 0, [], data, move |cx, path: &PathBuf, action| {
        if let Err(e) = cx.editor.open(path, action) {
            let err = if let Some(err) = e.source() {
                format!("{}", err)
            } else {
                format!("unable to open \"{}\"", path.display())
            };
            cx.editor.set_error(err);
        }
    })
    .with_preview(|_editor, path| Some((path.as_path().into(), None)));
    let injector = picker.injector();
    let timeout = std::time::Instant::now() + std::time::Duration::from_millis(30);

    let mut hit_timeout = false;
    for file in &mut files {
        if injector.push(file).is_err() {
            break;
        }
        if std::time::Instant::now() >= timeout {
            hit_timeout = true;
            break;
        }
    }
    if hit_timeout {
        std::thread::spawn(move || {
            for file in files {
                if injector.push(file).is_err() {
                    break;
                }
            }
        });
    }
    picker
}

type FileExplorer = Picker<(PathBuf, bool), (PathBuf, Style)>;

pub fn file_explorer(root: PathBuf, editor: &Editor) -> Result<FileExplorer, std::io::Error> {
    let directory_style = editor.theme.get("ui.text.directory");
    let directory_content = directory_content(&root, editor)?;

    let columns = [PickerColumn::new(
        "path",
        |(path, is_dir): &(PathBuf, bool), (root, directory_style): &(PathBuf, Style)| {
            let name = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
            if *is_dir {
                Span::styled(format!("{}/", name), *directory_style).into()
            } else {
                name.into()
            }
        },
    )];
    let picker = Picker::new(
        columns,
        0,
        directory_content,
        (root, directory_style),
        move |cx, (path, is_dir): &(PathBuf, bool), action| {
            if *is_dir {
                let new_root = helix_stdx::path::normalize(path);
                let callback = Box::pin(async move {
                    let call: Callback =
                        Callback::EditorCompositor(Box::new(move |editor, compositor| {
                            if let Ok(picker) = file_explorer(new_root, editor) {
                                compositor.push(Box::new(overlay::overlaid(picker)));
                            }
                        }));
                    Ok(call)
                });
                cx.jobs.callback(callback);
            } else if let Err(e) = cx.editor.open(path, action) {
                let err = if let Some(err) = e.source() {
                    format!("{}", err)
                } else {
                    format!("unable to open \"{}\"", path.display())
                };
                cx.editor.set_error(err);
            }
        },
    )
    .with_preview(|_editor, (path, _is_dir)| Some((path.as_path().into(), None)));

    Ok(picker)
}

fn directory_content(root: &Path, editor: &Editor) -> Result<Vec<(PathBuf, bool)>, std::io::Error> {
    use ignore::WalkBuilder;

    let config = editor.config();

    let mut walk_builder = WalkBuilder::new(root);

    let mut content: Vec<(PathBuf, bool)> = walk_builder
        .hidden(config.file_explorer.hidden)
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
                    let is_dir = entry
                        .file_type()
                        .is_some_and(|file_type| file_type.is_dir());
                    let mut path = entry.path().to_path_buf();
                    if is_dir && path != root && config.file_explorer.flatten_dirs {
                        while let Some(single_child_directory) = get_child_if_single_dir(&path) {
                            path = single_child_directory;
                        }
                    }
                    (path, is_dir)
                })
                .ok()
                .filter(|entry| entry.0 != root)
        })
        .collect();

    content.sort_by(|(path1, is_dir1), (path2, is_dir2)| (!is_dir1, path1).cmp(&(!is_dir2, path2)));

    if root.parent().is_some() {
        content.insert(0, (root.join(".."), true));
    }

    Ok(content)
}

fn get_child_if_single_dir(path: &Path) -> Option<PathBuf> {
    let mut entries = path.read_dir().ok()?;
    let entry = entries.next()?.ok()?;
    if entries.next().is_none() && entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
        Some(entry.path())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir, File};

    use super::*;

    #[test]
    fn test_get_child_if_single_dir() {
        let root = tempfile::tempdir().unwrap();

        assert_eq!(get_child_if_single_dir(root.path()), None);

        let dir = root.path().join("dir1");
        create_dir(&dir).unwrap();

        assert_eq!(get_child_if_single_dir(root.path()), Some(dir));

        let file = root.path().join("file");
        File::create(file).unwrap();

        assert_eq!(get_child_if_single_dir(root.path()), None);
    }
}
