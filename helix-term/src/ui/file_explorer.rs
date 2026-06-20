use std::error::Error as _;
use std::{
    fs,
    path::{Path, PathBuf},
};

use helix_core::hashmap;
use helix_view::{theme::Style, Editor};
use tui::text::Span;

use crate::{alt, compositor::Context, job::Callback};

use super::prompt::Movement;
use super::{
    directory_content, overlay, picker::PickerKeyHandler, Picker, PickerColumn, Prompt, PromptEvent,
};

/// for each path: (path to item, is the path a directory?)
type ExplorerItem = (PathBuf, bool);
/// (file explorer root, directory style)
type ExplorerData = (PathBuf, Style);

type FileExplorer = Picker<ExplorerItem, ExplorerData>;

type KeyHandler = PickerKeyHandler<ExplorerItem, ExplorerData>;

/// Create a prompt that asks for the user's confirmation before overwriting a path
fn confirm_before_overwriting<F>(
    // Path that we are overwriting
    overwriting: PathBuf,
    // Overwrite this path with
    overwrite_with: PathBuf,
    cx: &mut Context,
    picker_root: PathBuf,
    overwrite: F,
) -> Option<Result<String, String>>
where
    F: Fn(&mut Context, PathBuf, &Path) -> Option<Result<String, String>> + Send + 'static,
{
    // No need for confirmation, as the path does not exist. We can freely write to it
    if !overwriting.exists() {
        return overwrite(cx, picker_root, &overwrite_with);
    }
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |_editor, compositor| {
            let prompt = Prompt::new(
                format!(
                    "Path {} already exists. Ovewrite? (y/n):",
                    overwriting.display()
                )
                .into(),
                None,
                crate::ui::completers::none,
                move |cx, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate || input != "y" {
                        return;
                    };

                    if let Some(result) = overwrite(cx, picker_root.clone(), &overwrite_with) {
                        cx.editor.set_result(result);
                    };
                },
            );

            compositor.push(Box::new(prompt));
        }));
        Ok(call)
    });
    cx.jobs.callback(callback);

    None
}

fn create_file_operation_prompt<F>(
    cx: &mut Context,
    // Currently selected path of the picker
    path: &Path,
    // Text value of the prompt
    prompt: fn(&Path) -> String,
    // How to move the cursor
    movement: Option<Movement>,
    // What to fill user's input with
    prefill: fn(&Path) -> String,
    // Action to take when the operation runs
    file_op: F,
) where
    F: Fn(&mut Context, &PathBuf, String) -> Option<Result<String, String>> + Send + 'static,
{
    let selected_path = path.to_path_buf();
    let callback = Box::pin(async move {
        let call: Callback = Callback::EditorCompositor(Box::new(move |editor, compositor| {
            // to be able to move selected_path
            let path = selected_path.clone();
            let mut prompt = Prompt::new(
                prompt(&path).into(),
                None,
                crate::ui::completers::none,
                move |cx, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate {
                        return;
                    };

                    if let Some(result) = file_op(cx, &path, input.to_owned()) {
                        cx.editor.set_result(result);
                    } else {
                        cx.editor.clear_status();
                    };
                },
            );

            prompt.set_line(prefill(&selected_path), editor);

            if let Some(movement) = movement {
                log::error!("{movement:?}");
                prompt.move_cursor(movement);
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
    let directory_content = directory_content(&root, editor)?;

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
            cx,
            path,
            |_| "Create: ".into(),
            None,
            |path| {
                path.parent()
                    .map(|p| format!("{}{}", p.display(), std::path::MAIN_SEPARATOR))
                    .unwrap_or_default()
            },
            move |cx, _, to_create_string| {
                let root = data.0.clone();
                let to_create = helix_stdx::path::expand_tilde(PathBuf::from(&to_create_string));

                confirm_before_overwriting(
                    to_create.to_path_buf(),
                    to_create.to_path_buf(),
                    cx,
                    root,
                    move |cx: &mut Context, root: PathBuf, to_create: &Path| {
                        if to_create_string.ends_with(std::path::MAIN_SEPARATOR) {
                            if let Err(err_create_dir) =
                                fs::create_dir_all(to_create).map_err(|err| {
                                    format!(
                                        "Unable to create directory {}: {err}",
                                        to_create.display()
                                    )
                                })
                            {
                                return Some(Err(err_create_dir));
                            }
                            refresh_file_explorer(cursor, cx, root);

                            return Some(Ok(format!("Created directory: {}", to_create.display())));
                        }

                        // allows to create a path like /path/to/somewhere.txt even if "to" does not exist. Creates intermediate directories
                        let Some(to_create_parent) = to_create.parent() else {
                            return Some(Err(format!(
                                "Failed to get parent directory of {}",
                                to_create.display()
                            )));
                        };

                        if let Err(err_create_parent) = fs::create_dir_all(to_create_parent) {
                            return Some(Err(format!(
                                "Could not create intermediate directories: {err_create_parent}"
                            )));
                        }

                        if let Err(err_create_file) = fs::File::create(to_create).map_err(|err| {
                            format!("Unable to create file {}: {err}", to_create.display())
                        }) {
                            return Some(Err(err_create_file));
                        };

                        refresh_file_explorer(cursor, cx, root);

                        Some(Ok(format!("Created file: {}", to_create.display())))
                    },
                )
            },
        )
    });

    let move_: KeyHandler = Box::new(|cx, (path, _), data, cursor| {
        create_file_operation_prompt(
            cx,
            path,
            |path| format!("Move {} -> ", path.display()),
            // move cursor before the extension
            // Yazi does this and it leads to good user experience
            // Most of the time when we would like to rename a file we
            // don't want to change its file extension
            path.extension()
                .inspect(|a| log::error!("{a:?}"))
                // +1 to account for the dot in the extension (`.`)
                .map(|ext| Movement::BackwardChar(ext.len() + 1)),
            |path| path.display().to_string(),
            move |cx, move_from, move_to_string| {
                let root = data.0.clone();
                let move_to = helix_stdx::path::expand_tilde(PathBuf::from(&move_to_string));

                confirm_before_overwriting(
                    move_to.to_path_buf(),
                    move_from.to_path_buf(),
                    cx,
                    root,
                    move |cx: &mut Context, root: PathBuf, move_from: &Path| {
                        let move_to =
                            helix_stdx::path::expand_tilde(PathBuf::from(&move_to_string));

                        if let Err(err) = cx.editor.move_path(move_from, &move_to).map_err(|err| {
                            format!(
                                "Unable to move {} {} -> {}: {err}",
                                if move_to_string.ends_with(std::path::MAIN_SEPARATOR) {
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
                    },
                )
            },
        )
    });

    let delete: KeyHandler = Box::new(|cx, (path, _), data, cursor| {
        create_file_operation_prompt(
            cx,
            path,
            |path| format!("Delete {}? (y/n): ", path.display()),
            None,
            |_| "".to_string(),
            move |cx, to_delete, confirmation| {
                let root = data.0.clone();
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
            cx,
            path,
            |path| format!("Copy {} -> ", path.display()),
            None,
            |path| {
                path.parent()
                    .map(|p| format!("{}{}", p.display(), std::path::MAIN_SEPARATOR))
                    .unwrap_or_default()
            },
            move |cx, copy_from, copy_to_string| {
                let root = data.0.clone();
                let copy_to = helix_stdx::path::expand_tilde(PathBuf::from(&copy_to_string));

                if copy_from.is_dir() || copy_to_string.ends_with(std::path::MAIN_SEPARATOR) {
                    // TODO: support copying directories (recursively)?. This isn't built-in to the standard library
                    return Some(Err(format!(
                        "Copying directories is not supported: {} is a directory",
                        copy_from.display()
                    )));
                }

                let copy_to_str = copy_to_string.to_string();

                confirm_before_overwriting(
                    copy_to.to_path_buf(),
                    copy_from.to_path_buf(),
                    cx,
                    root,
                    move |cx: &mut Context, picker_root: PathBuf, copy_from: &Path| {
                        let copy_to = helix_stdx::path::expand_tilde(PathBuf::from(&copy_to_str));
                        if let Err(err) = std::fs::copy(copy_from, &copy_to).map_err(|err| {
                            format!(
                                "Unable to copy from file {} to {}: {err}",
                                copy_from.display(),
                                copy_to.display()
                            )
                        }) {
                            return Some(Err(err));
                        };
                        refresh_file_explorer(cursor, cx, picker_root);

                        Some(Ok(format!(
                            "Copied contents of file {} to {}",
                            copy_from.display(),
                            copy_to.display()
                        )))
                    },
                )
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
    })
    .with_title("File Explorer");

    Ok(picker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ===========================================
    // Path Validation Tests
    // ===========================================

    #[test]
    fn test_pathbuf_extension() {
        let path = PathBuf::from("/home/user/test.txt");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("txt"));
    }

    #[test]
    fn test_pathbuf_no_extension() {
        let path = PathBuf::from("/home/user/Makefile");
        assert!(path.extension().is_none());
    }

    #[test]
    fn test_pathbuf_hidden_file() {
        let path = PathBuf::from("/home/user/.gitignore");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(".gitignore")
        );
    }

    #[test]
    fn test_pathbuf_parent() {
        let path = PathBuf::from("/home/user/documents/file.txt");
        let parent = path.parent().unwrap();
        assert_eq!(parent, Path::new("/home/user/documents"));
    }

    #[test]
    fn test_pathbuf_is_absolute() {
        let abs_path = std::env::current_dir().unwrap().join("file.txt");
        let rel_path = PathBuf::from("./file.txt");

        assert!(abs_path.is_absolute());
        assert!(!rel_path.is_absolute());
    }

    // ===========================================
    // ExplorerItem Type Tests
    // ===========================================

    #[test]
    fn test_explorer_item_file() {
        let item: ExplorerItem = (PathBuf::from("/test/file.txt"), false);
        assert!(!item.1); // is_dir is false
        assert_eq!(item.0, PathBuf::from("/test/file.txt"));
    }

    #[test]
    fn test_explorer_item_directory() {
        let item: ExplorerItem = (PathBuf::from("/test/folder"), true);
        assert!(item.1); // is_dir is true
        assert_eq!(item.0, PathBuf::from("/test/folder"));
    }

    // ===========================================
    // Path Manipulation Tests
    // ===========================================

    #[test]
    fn test_path_join() {
        let base = PathBuf::from("/home/user");
        let joined = base.join("documents").join("file.txt");
        assert_eq!(joined, PathBuf::from("/home/user/documents/file.txt"));
    }

    #[test]
    fn test_path_strip_prefix() {
        let path = PathBuf::from("/home/user/documents/file.txt");
        let prefix = PathBuf::from("/home/user");
        let stripped = path.strip_prefix(&prefix).unwrap();
        assert_eq!(stripped, Path::new("documents/file.txt"));
    }

    #[test]
    fn test_path_strip_prefix_no_match() {
        let path = PathBuf::from("/home/user/documents/file.txt");
        let prefix = PathBuf::from("/var/log");
        let result = path.strip_prefix(&prefix);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_file_name() {
        let path = PathBuf::from("/home/user/documents/report.pdf");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("report.pdf")
        );
    }

    #[test]
    fn test_path_file_stem() {
        let path = PathBuf::from("/home/user/documents/report.pdf");
        assert_eq!(path.file_stem().and_then(|n| n.to_str()), Some("report"));
    }

    // ===========================================
    // Path Exists Tests (using temp paths)
    // ===========================================

    #[test]
    fn test_path_exists_false_for_nonexistent() {
        let path = PathBuf::from("/this/path/does/not/exist/hopefully");
        assert!(!path.exists());
    }

    #[test]
    fn test_path_exists_true_for_root() {
        // Root directory should always exist on Unix-like systems
        #[cfg(unix)]
        {
            let path = PathBuf::from("/");
            assert!(path.exists());
        }
    }

    // ===========================================
    // Directory Content Ordering Tests
    // ===========================================

    #[test]
    fn test_explorer_items_can_be_sorted() {
        let mut items: Vec<ExplorerItem> = vec![
            (PathBuf::from("z_file.txt"), false),
            (PathBuf::from("a_file.txt"), false),
            (PathBuf::from("m_folder"), true),
            (PathBuf::from("b_folder"), true),
        ];

        // Sort by name
        items.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(items[0].0, PathBuf::from("a_file.txt"));
        assert_eq!(items[1].0, PathBuf::from("b_folder"));
        assert_eq!(items[2].0, PathBuf::from("m_folder"));
        assert_eq!(items[3].0, PathBuf::from("z_file.txt"));
    }

    #[test]
    fn test_explorer_items_directories_first_sort() {
        let mut items: Vec<ExplorerItem> = vec![
            (PathBuf::from("z_file.txt"), false),
            (PathBuf::from("a_folder"), true),
            (PathBuf::from("a_file.txt"), false),
            (PathBuf::from("z_folder"), true),
        ];

        // Sort directories first, then by name
        items.sort_by(|a, b| match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(&b.0),
        });

        // Directories should come first
        assert!(items[0].1); // a_folder
        assert!(items[1].1); // z_folder
        assert!(!items[2].1); // a_file.txt
        assert!(!items[3].1); // z_file.txt
    }
}
