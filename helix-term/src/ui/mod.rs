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
use tui::text::Span;

use std::fs;
use std::path::Path;
use std::sync::Arc;
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

type FilePicker = Picker<PathBuf, PathBuf>;

pub fn file_picker(root: PathBuf, config: &helix_view::editor::Config) -> FilePicker {
    use ignore::{types::TypesBuilder, WalkBuilder};
    use std::time::Instant;

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
        |item: &PathBuf, root: &PathBuf| {
            item.strip_prefix(root)
                .unwrap_or(item)
                .to_string_lossy()
                .into()
        },
    )];
    let picker = Picker::new(columns, 0, [], root, move |cx, path: &PathBuf, action| {
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

/// for each path: (path to item, is the path a directory?)
type ExplorerItem = (PathBuf, bool);
/// (file explorer root, directory style)
type ExplorerData = (PathBuf, Style);

type FileExplorer = Picker<ExplorerItem, ExplorerData>;

type KeyHandler = PickerKeyHandler<ExplorerItem, ExplorerData>;

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

type FileOperation = fn(PathBuf, u32, &mut Context, &Path, &str) -> Option<Result<String, String>>;

fn create_file_operation_prompt(
    cursor: u32,
    prompt: fn(&Path) -> String,
    cx: &mut Context,
    path: &Path,
    data: Arc<ExplorerData>,
    compute_initial_line: fn(&Path) -> String,
    file_op: FileOperation,
) {
    cx.editor.file_explorer_selected_path = Some(path.to_path_buf());
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |editor, compositor| {
            let mut prompt = Prompt::new(
                editor
                    .file_explorer_selected_path
                    .as_ref()
                    .map(|p| prompt(p))
                    .unwrap_or_default()
                    .into(),
                None,
                crate::ui::completers::none,
                move |cx, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate {
                        return;
                    };

                    let path = cx.editor.file_explorer_selected_path.clone();

                    if let Some(path) = path {
                        match file_op(data.0.clone(), cursor, cx, &path, input) {
                            Some(Ok(msg)) => cx.editor.set_status(msg),
                            Some(Err(msg)) => cx.editor.set_error(msg),
                            None => cx.editor.clear_status(),
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

fn refresh_file_explorer(cursor: u32, cx: &mut Context, root: PathBuf) {
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |editor, compositor| {
            // replace the old file explorer with the new one
            compositor.pop();
            if let Ok(picker) = file_explorer(Some(cursor), root, editor) {
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

    let yank_path: KeyHandler = Box::new(|cx, (path, _), _, _| {
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
    });

    let create: KeyHandler = Box::new(|cx, (path, _), data, cursor| {
        create_file_operation_prompt(
            cursor,
            |_| "create:".into(),
            cx,
            path,
            data,
            |path| {
                path.parent()
                    .map(|p| format!("{}{}", p.display(), std::path::MAIN_SEPARATOR))
                    .unwrap_or_default()
            },
            |root, cursor, cx, _, to_create_str| {
                let to_create = helix_stdx::path::expand_tilde(PathBuf::from(to_create_str));

                let do_create = |cursor: u32,
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
                        refresh_file_explorer(cursor, cx, root);

                        return Some(Ok(format!("Created directory: {}", to_create.display())));
                    }

                    if let Err(err) = fs::File::create(to_create).map_err(|err| {
                        format!("Unable to create file {}: {err}", to_create.display())
                    }) {
                        return Some(Err(err));
                    };
                    refresh_file_explorer(cursor, cx, root);

                    Some(Ok(format!("Created file: {}", to_create.display())))
                };

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
                        do_create,
                    );
                    return None;
                };

                do_create(cursor, cx, root, to_create_str, &to_create)
            },
        )
    });

    let move_: KeyHandler = Box::new(|cx, (path, _), data, cursor| {
        create_file_operation_prompt(
            cursor,
            |path| format!("Move {} to:", path.display()),
            cx,
            path,
            data,
            |path| path.display().to_string(),
            |root, cursor, cx, move_from, move_to_str| {
                let move_to = helix_stdx::path::expand_tilde(PathBuf::from(move_to_str));

                let do_move = |cursor: u32,
                               cx: &mut Context,
                               root: PathBuf,
                               move_to_str: &str,
                               move_from: &Path| {
                    let move_to = helix_stdx::path::expand_tilde(PathBuf::from(move_to_str));

                    if let Err(err) = cx.editor.move_path(move_from, &move_to).map_err(|err| {
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
                    refresh_file_explorer(cursor, cx, root);
                    None
                };

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
                        do_move,
                    );
                    return None;
                };

                do_move(cursor, cx, root, move_to_str, move_from)
            },
        )
    });

    let delete: KeyHandler = Box::new(|cx, (path, _), data, cursor| {
        create_file_operation_prompt(
            cursor,
            |path| format!("Delete {}? (y/n):", path.display()),
            cx,
            path,
            data,
            |_| "".to_string(),
            |root, cursor, cx, to_delete, confirmation| {
                if confirmation != "y" {
                    return None;
                }

                if !to_delete.exists() {
                    return Some(Err(format!("Path {} does not exist", to_delete.display())));
                };

                if to_delete.is_dir() {
                    if let Err(err) = fs::remove_dir_all(to_delete).map_err(|err| {
                        format!("Unable to delete directory {}: {err}", to_delete.display())
                    }) {
                        return Some(Err(err));
                    };
                    refresh_file_explorer(cursor, cx, root);

                    return Some(Ok(format!("Deleted directory: {}", to_delete.display())));
                }

                if let Err(err) = fs::remove_file(to_delete)
                    .map_err(|err| format!("Unable to delete file {}: {err}", to_delete.display()))
                {
                    return Some(Err(err));
                };
                refresh_file_explorer(cursor, cx, root);

                Some(Ok(format!("Deleted file: {}", to_delete.display())))
            },
        )
    });

    let copy: KeyHandler = Box::new(|cx, (path, _), data, cursor| {
        create_file_operation_prompt(
            cursor,
            |path| format!("Copy {} to:", path.display()),
            cx,
            path,
            data,
            |path| {
                path.parent()
                    .map(|p| format!("{}{}", p.display(), std::path::MAIN_SEPARATOR))
                    .unwrap_or_default()
            },
            |root, cursor, cx, copy_from, copy_to_str| {
                let copy_to = helix_stdx::path::expand_tilde(PathBuf::from(copy_to_str));

                let do_copy = |cursor: u32,
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
                    refresh_file_explorer(cursor, cx, root);

                    Some(Ok(format!(
                        "Copied contents of file {} to {}",
                        copy_from.display(),
                        copy_to.display()
                    )))
                };

                if copy_from.is_dir() || copy_to_str.ends_with(std::path::MAIN_SEPARATOR) {
                    // TODO: support copying directories (recursively)?. This isn't built-in to the standard library
                    return Some(Err(format!(
                        "Copying directories is not supported: {} is a directory",
                        copy_from.display()
                    )));
                }

                if copy_to.exists() {
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
                        do_copy,
                    );
                    return None;
                }

                do_copy(cursor, cx, root, copy_to_str, copy_from)
            },
        )
    });

    let columns = [PickerColumn::new(
        "path",
        |(path, is_dir): &ExplorerItem, (root, directory_style): &ExplorerData| {
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
        move |cx, (path, is_dir): &ExplorerItem, action| {
            if *is_dir {
                let new_root = helix_stdx::path::normalize(path);
                let callback = Box::pin(async move {
                    let call: Callback =
                        Callback::EditorCompositor(Box::new(move |editor, compositor| {
                            if let Ok(picker) = file_explorer(None, new_root, editor) {
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
    .with_cursor(cursor.unwrap_or_default())
    .with_preview(|_editor, (path, _is_dir)| Some((path.as_path().into(), None)))
    .with_key_handlers(hashmap! {
        alt!('n') => create,
        alt!('m') => move_,
        alt!('d') => delete,
        alt!('c') => copy,
        alt!('y') => yank_path,
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
    use helix_core::fuzzy::fuzzy_match;
    use helix_core::syntax::LanguageServerFeature;
    use helix_view::document::SCRATCH_BUFFER_NAME;
    use helix_view::theme;
    use helix_view::{editor::Config, Editor};
    use once_cell::sync::Lazy;
    use std::borrow::Cow;
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

    pub fn language_servers(editor: &Editor, input: &str) -> Vec<Completion> {
        let language_servers = doc!(editor).language_servers().map(|ls| ls.name());

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
}
