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
mod spinner;
mod statusline;
mod text;
mod text_decorations;

use crate::compositor::{Compositor, Context};
use crate::job::{self, Callback};
use crate::{alt, filter_picker_entry};
pub use completion::Completion;
pub use editor::EditorView;
use helix_core::hashmap;
use helix_stdx::rope;
use helix_view::theme::Style;
pub use markdown::Markdown;
pub use menu::Menu;
pub use picker::{Column as PickerColumn, FileLocation, Picker};
pub use popup::Popup;
pub use prompt::{Prompt, PromptEvent};
pub use spinner::{ProgressSpinners, Spinner};
pub use text::Text;

use helix_view::Editor;
use tui::text::{Span, Spans};

use std::fs;
use std::path::Path;
use std::{error::Error, path::PathBuf};

use self::picker::PickerKeyHandler;

struct Utf8PathBuf {
    path: String,
    is_dir: bool,
}

impl AsRef<str> for Utf8PathBuf {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

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

#[derive(Debug)]
pub struct FilePickerData {
    root: PathBuf,
    directory_style: Style,
}
type FilePicker = Picker<PathBuf, FilePickerData>;

pub fn file_picker(editor: &Editor, root: PathBuf) -> FilePicker {
    use ignore::{types::TypesBuilder, WalkBuilder};
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
    walk_builder
        .hidden(config.file_picker.hidden)
        .parents(config.file_picker.parents)
        .ignore(config.file_picker.ignore)
        .follow_links(config.file_picker.follow_symlinks)
        .git_ignore(config.file_picker.git_ignore)
        .git_global(config.file_picker.git_global)
        .git_exclude(config.file_picker.git_exclude)
        .sort_by_file_name(|name1, name2| name1.cmp(name2))
        .max_depth(config.file_picker.max_depth)
        .filter_entry(move |entry| filter_picker_entry(entry, &absolute_root, dedup_symlinks));

    walk_builder.add_custom_ignore_filename(helix_loader::config_dir().join("ignore"));
    walk_builder.add_custom_ignore_filename(".helix/ignore");

    // We want to exclude files that the editor can't handle yet
    let mut type_builder = TypesBuilder::new();
    type_builder
        .add(
            "compressed",
            "*.{zip,gz,bz2,zst,lzo,sz,tgz,tbz2,lz,lz4,lzma,lzo,z,Z,xz,7z,rar,cab}",
        )
        .expect("Invalid type definition");
    type_builder.negate("all");
    let excluded_types = type_builder
        .build()
        .expect("failed to build excluded_types");
    walk_builder.types(excluded_types);
    let mut files = walk_builder.build().filter_map(|entry| {
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

type OnConfirm = fn(
    cursor: u32,
    cx: &mut Context,
    picker_root: PathBuf,
    &str,
    &Path,
) -> Option<Result<String, String>>;

fn create_confirmation_prompt(
    cursor: u32,
    input: String,
    cx: &mut Context,
    operation_input_str: String,
    operation_input: PathBuf,
    picker_root: PathBuf,
    on_confirm: OnConfirm,
) {
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |_editor, compositor| {
            let prompt = Prompt::new(
                input.into(),
                None,
                crate::ui::completers::none,
                move |cx, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate || input != "y" {
                        return;
                    };

                    match on_confirm(
                        cursor,
                        cx,
                        picker_root.clone(),
                        &operation_input_str,
                        &operation_input,
                    ) {
                        Some(Ok(msg)) => cx.editor.set_status(msg),
                        Some(Err(msg)) => cx.editor.set_error(msg),
                        None => (),
                    };
                },
            );

            compositor.push(Box::new(prompt));
        }));
        Ok(call)
    });
    cx.jobs.callback(callback);
}

type FileOperation = fn(u32, &mut Context, &Path, &str) -> Option<Result<String, String>>;

fn create_file_operation_prompt(
    cursor: u32,
    prompt: &'static str,
    cx: &mut Context,
    path: &Path,
    compute_initial_line: fn(&Path) -> String,
    file_op: FileOperation,
) {
    cx.editor.file_explorer_selected_path = Some(path.to_path_buf());
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |editor, compositor| {
            let mut prompt = Prompt::new(
                prompt.into(),
                None,
                crate::ui::completers::none,
                move |cx, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate {
                        return;
                    };

                    let path = cx.editor.file_explorer_selected_path.clone();

                    if let Some(path) = path {
                        match file_op(cursor, cx, &path, input) {
                            Some(Ok(msg)) => cx.editor.set_status(msg),
                            Some(Err(msg)) => cx.editor.set_error(msg),
                            None => (),
                        };
                    } else {
                        cx.editor
                            .set_error("Unable to determine path of selected file")
                    }
                },
            );

            if let Some(path_editing) = &editor.file_explorer_selected_path {
                prompt.set_line_no_recalculate(compute_initial_line(path_editing));
            }

            compositor.push(Box::new(prompt));
        }));
        Ok(call)
    });
    cx.jobs.callback(callback);
}

fn refresh_file_explorer(
    remove_previous: bool,
    cursor: Option<u32>,
    cx: &mut Context,
    root: PathBuf,
) {
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |editor, compositor| {
            if remove_previous {
                compositor.pop();
            }
            if let Ok(picker) = file_explorer(cursor, root, editor) {
                compositor.push(Box::new(overlay::overlaid(picker)));
            }
        }));
        Ok(call)
    });
    cx.jobs.callback(callback);
}

pub fn file_explorer(
    cursor: Option<u32>,
    root: PathBuf,
    editor: &Editor,
) -> Result<FileExplorer, std::io::Error> {
    let directory_style = editor.theme.get("ui.text.directory");
    let directory_content = directory_content(&root)?;

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

    let yank_path = |cx: &mut Context, (path, _is_dir): &(PathBuf, bool), _cursor: u32| {
        let register = cx
            .editor
            .selected_register
            .unwrap_or(cx.editor.config().default_yank_register);
        let path = helix_stdx::path::get_relative_path(path);
        let path = path.to_string_lossy().to_string();
        let message = format!("Yanked path {} to register {register}", path);

        match cx.editor.registers.write(register, vec![path]) {
            Ok(()) => cx.editor.set_status(message),
            Err(err) => cx.editor.set_error(err.to_string()),
        };
    };

    let create_file = |cx: &mut Context, (path, _is_dir): &(PathBuf, bool), cursor: u32| {
        create_file_operation_prompt(
            cursor,
            "create:",
            cx,
            path,
            |path| {
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            },
            |cursor, cx, path, to_create_str| {
                let to_create = helix_stdx::path::expand_tilde(PathBuf::from(to_create_str));

                let create_op = |cursor: u32,
                                 cx: &mut Context,
                                 root: PathBuf,
                                 to_create_str: &str,
                                 to_create: &Path| {
                    if to_create_str.ends_with(std::path::MAIN_SEPARATOR) {
                        if let Err(err) = fs::create_dir_all(to_create).map_err(|err| {
                            format!("Unable to create directory {}: {err}", to_create.display())
                        }) {
                            return Some(Err(err));
                        }
                        refresh_file_explorer(true, Some(cursor), cx, root);

                        Some(Ok(format!("Created directory: {}", to_create.display())))
                    } else {
                        if let Err(err) = fs::File::create(to_create).map_err(|err| {
                            format!("Unable to create file {}: {err}", to_create.display())
                        }) {
                            return Some(Err(err));
                        };
                        refresh_file_explorer(true, Some(cursor), cx, root);

                        Some(Ok(format!("Created file: {}", to_create.display())))
                    }
                };

                let root = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(helix_stdx::env::current_working_dir());

                if to_create.exists() {
                    create_confirmation_prompt(
                        cursor,
                        format!(
                            "Path {} already exists. Overwrite? (y/n):",
                            to_create.display()
                        ),
                        cx,
                        to_create_str.to_string(),
                        to_create.to_path_buf(),
                        root,
                        create_op,
                    );
                    return None;
                };

                create_op(cursor, cx, root, to_create_str, &to_create)
            },
        )
    };

    let move_file = |cx: &mut Context, (path, _is_dir): &(PathBuf, bool), cursor: u32| {
        create_file_operation_prompt(
            cursor,
            "move:",
            cx,
            path,
            |path| path.display().to_string(),
            |cursor, cx, move_from, move_to_str| {
                let move_to = helix_stdx::path::expand_tilde(PathBuf::from(move_to_str));

                let move_op = |cursor: u32,
                               cx: &mut Context,
                               root: PathBuf,
                               move_to_str: &str,
                               move_from: &Path| {
                    let move_to = helix_stdx::path::expand_tilde(PathBuf::from(move_to_str));
                    if let Err(err) = fs::rename(move_from, &move_to).map_err(|err| {
                        format!(
                            "Unable to move {} {} -> {}: {err}",
                            if move_to_str.ends_with(std::path::MAIN_SEPARATOR) {
                                "directory"
                            } else {
                                "file"
                            },
                            move_from.display(),
                            move_to.display()
                        )
                    }) {
                        return Some(Err(err));
                    };
                    refresh_file_explorer(true, Some(cursor), cx, root);
                    None
                };

                let root = move_from
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(helix_stdx::env::current_working_dir());

                if move_to.exists() {
                    create_confirmation_prompt(
                        cursor,
                        format!(
                            "Path {} already exists. Overwrite? (y/n):",
                            move_to.display()
                        ),
                        cx,
                        move_to_str.to_string(),
                        move_from.to_path_buf(),
                        root,
                        move_op,
                    );
                    return None;
                };

                move_op(cursor, cx, root, move_to_str, move_from)
            },
        )
    };

    let delete_file = |cx: &mut Context, (path, _is_dir): &(PathBuf, bool), cursor: u32| {
        create_file_operation_prompt(
            cursor,
            "delete? (y/n):",
            cx,
            path,
            |_| "".to_string(),
            |cursor, cx, to_delete, confirmation| {
                if confirmation == "y" {
                    if !to_delete.exists() {
                        return Some(Err(format!("Path {} does not exist", to_delete.display())));
                    };

                    let root = to_delete
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or(helix_stdx::env::current_working_dir());

                    if confirmation.ends_with(std::path::MAIN_SEPARATOR) {
                        if let Err(err) = fs::remove_dir_all(to_delete).map_err(|err| {
                            format!("Unable to delete directory {}: {err}", to_delete.display())
                        }) {
                            return Some(Err(err));
                        };
                        refresh_file_explorer(true, Some(cursor), cx, root);

                        Some(Ok(format!("Deleted directory: {}", to_delete.display())))
                    } else {
                        if let Err(err) = fs::remove_file(to_delete).map_err(|err| {
                            format!("Unable to delete file {}: {err}", to_delete.display())
                        }) {
                            return Some(Err(err));
                        };
                        refresh_file_explorer(true, Some(cursor), cx, root);

                        Some(Ok(format!("Deleted file: {}", to_delete.display())))
                    }
                } else {
                    None
                }
            },
        )
    };

    let copy_file = |cx: &mut Context, (path, _is_dir): &(PathBuf, bool), cursor: u32| {
        create_file_operation_prompt(
            cursor,
            "copy-to:",
            cx,
            path,
            |path| {
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            },
            |cursor, cx, copy_from, copy_to_str| {
                let copy_to = helix_stdx::path::expand_tilde(PathBuf::from(copy_to_str));

                let copy_op = |cursor: u32,
                               cx: &mut Context,
                               root: PathBuf,
                               copy_to_str: &str,
                               copy_from: &Path| {
                    let copy_to = helix_stdx::path::expand_tilde(PathBuf::from(copy_to_str));
                    if let Err(err) = std::fs::copy(copy_from, &copy_to).map_err(|err| {
                        format!(
                            "Unable to copy from file {} to {}: {err}",
                            copy_from.display(),
                            copy_to.display()
                        )
                    }) {
                        return Some(Err(err));
                    };
                    refresh_file_explorer(true, Some(cursor), cx, root);

                    Some(Ok(format!(
                        "Copied contents of file {} to {}",
                        copy_from.display(),
                        copy_to.display()
                    )))
                };

                let root = copy_to
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(helix_stdx::env::current_working_dir());

                if copy_from.is_dir() || copy_to_str.ends_with(std::path::MAIN_SEPARATOR) {
                    // TODO: support copying directories (recursively)?. This isn't built-in to the standard library
                    Some(Err(format!(
                        "Copying directories is not supported: {} is a directory",
                        copy_from.display()
                    )))
                } else if copy_to.exists() {
                    create_confirmation_prompt(
                        cursor,
                        format!(
                            "Path {} already exists. Overwrite? (y/n):",
                            copy_to.display()
                        ),
                        cx,
                        copy_to_str.to_string(),
                        copy_from.to_path_buf(),
                        root,
                        copy_op,
                    );
                    None
                } else {
                    copy_op(cursor, cx, root, copy_to_str, copy_from)
                }
            },
        )
    };

    type KeyHandler = PickerKeyHandler<(PathBuf, bool)>;

    let picker = Picker::new(
        columns,
        0,
        directory_content,
        (root, directory_style),
        move |cx, (path, is_dir): &(PathBuf, bool), action| {
            if *is_dir {
                let new_root = helix_stdx::path::normalize(path);
                refresh_file_explorer(false, None, cx, new_root);
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
    .with_cursor(cursor.unwrap_or_default())
    .with_preview(|_editor, (path, _is_dir)| Some((path.as_path().into(), None)))
    .with_key_handlers(hashmap! {
        alt!('n') => Box::new(create_file) as KeyHandler,
        alt!('m') => Box::new(move_file) as KeyHandler,
        alt!('d') => Box::new(delete_file) as KeyHandler,
        alt!('c') => Box::new(copy_file) as KeyHandler,
        alt!('y') => Box::new(yank_path) as KeyHandler,
    });

    Ok(picker)
}

fn directory_content(path: &Path) -> Result<Vec<(PathBuf, bool)>, std::io::Error> {
    let mut content: Vec<_> = std::fs::read_dir(path)?
        .flatten()
        .map(|entry| {
            (
                entry.path(),
                entry.file_type().is_ok_and(|file_type| file_type.is_dir()),
            )
        })
        .collect();

    content.sort_by(|(path1, is_dir1), (path2, is_dir2)| (!is_dir1, path1).cmp(&(!is_dir2, path2)));
    if path.parent().is_some() {
        content.insert(0, (path.join(".."), true));
    }
    Ok(content)
}

pub mod completers {
    use super::Utf8PathBuf;
    use crate::ui::prompt::Completion;
    use helix_core::command_line::{self, Tokenizer};
    use helix_core::fuzzy::fuzzy_match;
    use helix_core::syntax::config::LanguageServerFeature;
    use helix_view::document::SCRATCH_BUFFER_NAME;
    use helix_view::theme;
    use helix_view::{editor::Config, Editor};
    use once_cell::sync::Lazy;
    use std::borrow::Cow;
    use std::collections::BTreeSet;
    use tui::text::Span;

    pub type Completer = fn(&Editor, &str) -> Vec<Completion>;

    pub fn none(_editor: &Editor, _input: &str) -> Vec<Completion> {
        Vec::new()
    }

    pub fn buffer(editor: &Editor, input: &str) -> Vec<Completion> {
        let names = editor.documents.values().map(|doc| {
            doc.relative_path()
                .map(|p| p.display().to_string().into())
                .unwrap_or_else(|| Cow::from(SCRATCH_BUFFER_NAME))
        });

        fuzzy_match(input, names, true)
            .into_iter()
            .map(|(name, _)| ((0..), name.into()))
            .collect()
    }

    pub fn theme(_editor: &Editor, input: &str) -> Vec<Completion> {
        let mut names = theme::Loader::read_names(&helix_loader::config_dir().join("themes"));
        for rt_dir in helix_loader::runtime_dirs() {
            names.extend(theme::Loader::read_names(&rt_dir.join("themes")));
        }
        names.push("default".into());
        names.push("base16_default".into());
        names.sort();
        names.dedup();

        fuzzy_match(input, names, false)
            .into_iter()
            .map(|(name, _)| ((0..), name.into()))
            .collect()
    }

    /// Recursive function to get all keys from this value and add them to vec
    fn get_keys(value: &serde_json::Value, vec: &mut Vec<String>, scope: Option<&str>) {
        if let Some(map) = value.as_object() {
            for (key, value) in map.iter() {
                let key = match scope {
                    Some(scope) => format!("{}.{}", scope, key),
                    None => key.clone(),
                };
                get_keys(value, vec, Some(&key));
                if !value.is_object() {
                    vec.push(key);
                }
            }
        }
    }

    /// Completes names of language servers which are running for the current document.
    pub fn active_language_servers(editor: &Editor, input: &str) -> Vec<Completion> {
        let language_servers = doc!(editor).language_servers().map(|ls| ls.name());

        fuzzy_match(input, language_servers, false)
            .into_iter()
            .map(|(name, _)| ((0..), Span::raw(name.to_string())))
            .collect()
    }

    /// Completes names of language servers which are configured for the language of the current
    /// document.
    pub fn configured_language_servers(editor: &Editor, input: &str) -> Vec<Completion> {
        let language_servers = doc!(editor)
            .language_config()
            .into_iter()
            .flat_map(|config| &config.language_servers)
            .map(|ls| ls.name.as_str());

        fuzzy_match(input, language_servers, false)
            .into_iter()
            .map(|(name, _)| ((0..), Span::raw(name.to_string())))
            .collect()
    }

    pub fn setting(_editor: &Editor, input: &str) -> Vec<Completion> {
        static KEYS: Lazy<Vec<String>> = Lazy::new(|| {
            let mut keys = Vec::new();
            let json = serde_json::json!(Config::default());
            get_keys(&json, &mut keys, None);
            keys
        });

        fuzzy_match(input, &*KEYS, false)
            .into_iter()
            .map(|(name, _)| ((0..), Span::raw(name)))
            .collect()
    }

    pub fn filename(editor: &Editor, input: &str) -> Vec<Completion> {
        filename_with_git_ignore(editor, input, true)
    }

    pub fn filename_with_git_ignore(
        editor: &Editor,
        input: &str,
        git_ignore: bool,
    ) -> Vec<Completion> {
        filename_impl(editor, input, git_ignore, |entry| {
            let is_dir = entry.file_type().is_some_and(|entry| entry.is_dir());

            if is_dir {
                FileMatch::AcceptIncomplete
            } else {
                FileMatch::Accept
            }
        })
    }

    pub fn language(editor: &Editor, input: &str) -> Vec<Completion> {
        let text: String = "text".into();

        let loader = editor.syn_loader.load();
        let language_ids = loader
            .language_configs()
            .map(|config| &config.language_id)
            .chain(std::iter::once(&text));

        fuzzy_match(input, language_ids, false)
            .into_iter()
            .map(|(name, _)| ((0..), name.to_owned().into()))
            .collect()
    }

    pub fn lsp_workspace_command(editor: &Editor, input: &str) -> Vec<Completion> {
        let commands = doc!(editor)
            .language_servers_with_feature(LanguageServerFeature::WorkspaceCommand)
            .flat_map(|ls| {
                ls.capabilities()
                    .execute_command_provider
                    .iter()
                    .flat_map(|options| options.commands.iter())
            });

        fuzzy_match(input, commands, false)
            .into_iter()
            .map(|(name, _)| ((0..), name.to_owned().into()))
            .collect()
    }

    pub fn directory(editor: &Editor, input: &str) -> Vec<Completion> {
        directory_with_git_ignore(editor, input, true)
    }

    pub fn directory_with_git_ignore(
        editor: &Editor,
        input: &str,
        git_ignore: bool,
    ) -> Vec<Completion> {
        filename_impl(editor, input, git_ignore, |entry| {
            let is_dir = entry.file_type().is_some_and(|entry| entry.is_dir());

            if is_dir {
                FileMatch::Accept
            } else {
                FileMatch::Reject
            }
        })
    }

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum FileMatch {
        /// Entry should be ignored
        Reject,
        /// Entry is usable but can't be the end (for instance if the entry is a directory and we
        /// try to match a file)
        AcceptIncomplete,
        /// Entry is usable and can be the end of the match
        Accept,
    }

    // TODO: we could return an iter/lazy thing so it can fetch as many as it needs.
    fn filename_impl<F>(
        editor: &Editor,
        input: &str,
        git_ignore: bool,
        filter_fn: F,
    ) -> Vec<Completion>
    where
        F: Fn(&ignore::DirEntry) -> FileMatch,
    {
        // Rust's filename handling is really annoying.

        use ignore::WalkBuilder;
        use std::path::Path;

        let is_tilde = input == "~";
        let path = helix_stdx::path::expand_tilde(Path::new(input));

        let (dir, file_name) = if input.ends_with(std::path::MAIN_SEPARATOR) {
            (path, None)
        } else {
            let is_period = (input.ends_with((format!("{}.", std::path::MAIN_SEPARATOR)).as_str())
                && input.len() > 2)
                || input == ".";
            let file_name = if is_period {
                Some(String::from("."))
            } else {
                path.file_name()
                    .and_then(|file| file.to_str().map(|path| path.to_owned()))
            };

            let path = if is_period {
                path
            } else {
                match path.parent() {
                    Some(path) if !path.as_os_str().is_empty() => Cow::Borrowed(path),
                    // Path::new("h")'s parent is Some("")...
                    _ => Cow::Owned(helix_stdx::env::current_working_dir()),
                }
            };

            (path, file_name)
        };

        let end = input.len()..;

        let files = WalkBuilder::new(&dir)
            .hidden(false)
            .follow_links(false) // We're scanning over depth 1
            .git_ignore(git_ignore)
            .max_depth(Some(1))
            .build()
            .filter_map(|file| {
                file.ok().and_then(|entry| {
                    let fmatch = filter_fn(&entry);

                    if fmatch == FileMatch::Reject {
                        return None;
                    }

                    let is_dir = entry.file_type().is_some_and(|entry| entry.is_dir());

                    let path = entry.path();
                    let mut path = if is_tilde {
                        // if it's a single tilde an absolute path is displayed so that when `TAB` is pressed on
                        // one of the directories the tilde will be replaced with a valid path not with a relative
                        // home directory name.
                        // ~ -> <TAB> -> /home/user
                        // ~/ -> <TAB> -> ~/first_entry
                        path.to_path_buf()
                    } else {
                        path.strip_prefix(&dir).unwrap_or(path).to_path_buf()
                    };

                    if fmatch == FileMatch::AcceptIncomplete {
                        path.push("");
                    }

                    let path = path.into_os_string().into_string().ok()?;
                    Some(Utf8PathBuf { path, is_dir })
                })
            }) // TODO: unwrap or skip
            .filter(|path| !path.path.is_empty());

        let directory_color = editor.theme.get("ui.text.directory");

        let style_from_file = |file: Utf8PathBuf| {
            if file.is_dir {
                Span::styled(file.path, directory_color)
            } else {
                Span::raw(file.path)
            }
        };

        // if empty, return a list of dirs and files in current dir
        if let Some(file_name) = file_name {
            let range = (input.len().saturating_sub(file_name.len()))..;
            fuzzy_match(&file_name, files, true)
                .into_iter()
                .map(|(name, _)| (range.clone(), style_from_file(name)))
                .collect()

            // TODO: complete to longest common match
        } else {
            let mut files: Vec<_> = files
                .map(|file| (end.clone(), style_from_file(file)))
                .collect();
            files.sort_unstable_by(|(_, path1), (_, path2)| path1.content.cmp(&path2.content));
            files
        }
    }

    pub fn register(editor: &Editor, input: &str) -> Vec<Completion> {
        let iter = editor
            .registers
            .iter_preview()
            // Exclude special registers that shouldn't be written to
            .filter(|(ch, _)| !matches!(ch, '%' | '#' | '.'))
            .map(|(ch, _)| ch.to_string());

        fuzzy_match(input, iter, false)
            .into_iter()
            .map(|(name, _)| ((0..), name.into()))
            .collect()
    }

    pub fn program(_editor: &Editor, input: &str) -> Vec<Completion> {
        static PROGRAMS_IN_PATH: Lazy<BTreeSet<String>> = Lazy::new(|| {
            // Go through the entire PATH and read all files into a set.
            let Some(path) = std::env::var_os("PATH") else {
                return Default::default();
            };

            std::env::split_paths(&path)
                .filter_map(|path| std::fs::read_dir(path).ok())
                .flatten()
                .filter_map(|res| {
                    let entry = res.ok()?;
                    if entry.metadata().ok()?.is_file() {
                        entry.file_name().into_string().ok()
                    } else {
                        None
                    }
                })
                .collect()
        });

        fuzzy_match(input, PROGRAMS_IN_PATH.iter(), false)
            .into_iter()
            .map(|(name, _)| ((0..), name.clone().into()))
            .collect()
    }

    /// This expects input to be a raw string of arguments, because this is what Signature's raw_after does.
    pub fn repeating_filenames(editor: &Editor, input: &str) -> Vec<Completion> {
        let token = match Tokenizer::new(input, false).last() {
            Some(token) => token.unwrap(),
            None => return filename(editor, input),
        };

        let offset = token.content_start;

        let mut completions = filename(editor, &input[offset..]);
        for completion in completions.iter_mut() {
            completion.0.start += offset;
        }
        completions
    }

    pub fn shell(editor: &Editor, input: &str) -> Vec<Completion> {
        let (command, args, complete_command) = command_line::split(input);

        if complete_command {
            return program(editor, command);
        }

        let mut completions = repeating_filenames(editor, args);
        for completion in completions.iter_mut() {
            // + 1 for separator between `command` and `args`
            completion.0.start += command.len() + 1;
        }

        completions
    }
}
