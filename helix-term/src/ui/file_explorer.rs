//! File explorer with file management capabilities.
//!
//! Provides create, delete, yank (copy), and paste operations with live feedback.

use std::path::{Path, PathBuf};

use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Rect},
    theme::Style,
    Editor,
};
use tui::buffer::Buffer as Surface;

use crate::{
    compositor::{self, Component, Compositor, Context, Event, EventResult},
    job::Callback as JobCallback,
    key,
    ui::{self, overlay, Picker, PickerColumn, Prompt, PromptEvent},
};

pub const ID: &str = "file-explorer";

/// Clipboard entry for copy/paste operations
#[derive(Clone)]
pub struct ClipboardEntry {
    pub path: PathBuf,
}

/// Global clipboard for file operations (shared across file explorer instances)
static CLIPBOARD: std::sync::OnceLock<std::sync::Mutex<Option<ClipboardEntry>>> =
    std::sync::OnceLock::new();

fn get_clipboard() -> &'static std::sync::Mutex<Option<ClipboardEntry>> {
    CLIPBOARD.get_or_init(|| std::sync::Mutex::new(None))
}

/// File explorer with file management capabilities
pub struct FileExplorer {
    picker: Picker<(PathBuf, bool), (PathBuf, Style)>,
    root: PathBuf,
}

impl FileExplorer {
    pub fn new(root: PathBuf, editor: &Editor) -> Result<Self, std::io::Error> {
        let picker = create_picker(root.clone(), editor)?;
        Ok(Self { picker, root })
    }

    /// Get the currently selected path, if any
    fn selected_path(&self) -> Option<&(PathBuf, bool)> {
        self.picker.selection()
    }

    /// Get the current directory (either selected directory or root)
    fn current_directory(&self) -> PathBuf {
        if let Some((path, is_dir)) = self.selected_path() {
            if *is_dir && path.file_name().map(|n| n != "..").unwrap_or(false) {
                return path.clone();
            }
        }
        self.root.clone()
    }

    /// Handle 'a' key - create file or directory
    fn handle_create(&mut self, _ctx: &mut Context) -> EventResult {
        let current_dir = self.current_directory();
        let root = self.root.clone();

        let prompt = Prompt::new(
            "new: ".into(),
            None,
            ui::completers::none,
            move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                if event != PromptEvent::Validate {
                    return;
                }

                if input.is_empty() {
                    return;
                }

                let path = current_dir.join(input);
                let is_dir = input.ends_with('/') || input.ends_with(std::path::MAIN_SEPARATOR);

                let result = if is_dir {
                    // Create directory (strip trailing slash for the actual path)
                    let dir_path = path.with_file_name(
                        path.file_name()
                            .map(|n| n.to_string_lossy().trim_end_matches('/').trim_end_matches(std::path::MAIN_SEPARATOR).to_string())
                            .unwrap_or_default(),
                    );
                    std::fs::create_dir_all(&dir_path).map(|_| dir_path)
                } else {
                    // Create file (and parent directories if needed)
                    if let Some(parent) = path.parent() {
                        if !parent.exists() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                cx.editor.set_error(format!("Failed to create directories: {}", e));
                                return;
                            }
                        }
                    }
                    std::fs::File::create(&path).map(|_| path.clone())
                };

                match result {
                    Ok(created_path) => {
                        cx.editor.set_status(format!("Created: {}", created_path.display()));
                        schedule_refresh_with_prompt(cx, root.clone());
                    }
                    Err(e) => {
                        cx.editor.set_error(format!("Failed to create: {}", e));
                    }
                }
            },
        );

        EventResult::Consumed(Some(Box::new(move |compositor: &mut Compositor, _cx: &mut Context| {
            compositor.push(Box::new(prompt));
        })))
    }

    /// Handle 'd' key - delete file or directory
    fn handle_delete(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow deleting ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor.set_error("Cannot delete parent directory reference");
            return EventResult::Consumed(None);
        }

        let path_display = path.display().to_string();
        let prompt_text = if is_dir {
            format!("Delete directory '{}'? (y/N): ", path_display)
        } else {
            format!("Delete '{}'? (y/N): ", path_display)
        };

        let root = self.root.clone();

        let prompt = Prompt::new(
            prompt_text.into(),
            None,
            ui::completers::none,
            move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                if event != PromptEvent::Validate {
                    return;
                }

                let response = input.trim().to_lowercase();
                if response != "y" && response != "yes" {
                    cx.editor.set_status("Delete cancelled");
                    return;
                }

                let result = if is_dir {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };

                match result {
                    Ok(_) => {
                        cx.editor.set_status(format!("Deleted: {}", path_display));
                        schedule_refresh_with_prompt(cx, root.clone());
                    }
                    Err(e) => {
                        cx.editor.set_error(format!("Failed to delete: {}", e));
                    }
                }
            },
        );

        EventResult::Consumed(Some(Box::new(move |compositor: &mut Compositor, _cx: &mut Context| {
            compositor.push(Box::new(prompt));
        })))
    }

    /// Handle 'y' key - yank (copy) file/directory to clipboard
    fn handle_yank(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, _is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow yanking ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor.set_error("Cannot yank parent directory reference");
            return EventResult::Consumed(None);
        }

        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = Some(ClipboardEntry { path: path.clone() });
        }
        ctx.editor.set_status(format!("Yanked: {}", path.display()));
        EventResult::Consumed(None)
    }

    /// Handle 'p' key - paste from clipboard
    fn handle_paste(&mut self, ctx: &mut Context) -> EventResult {
        let clipboard_entry = get_clipboard()
            .lock()
            .ok()
            .and_then(|guard| guard.clone());

        let Some(entry) = clipboard_entry else {
            ctx.editor.set_error("Clipboard is empty");
            return EventResult::Consumed(None);
        };

        let dest_dir = self.current_directory();
        let file_name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        let dest_path = dest_dir.join(&file_name);
        let root = self.root.clone();
        let src_path = entry.path.clone();
        let is_dir = src_path.is_dir();

        // Check if destination already exists
        if dest_path.exists() {
            let prompt_text = format!("'{}' exists. Overwrite? (y/N): ", file_name);

            let prompt = Prompt::new(
                prompt_text.into(),
                None,
                ui::completers::none,
                move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate {
                        return;
                    }

                    let response = input.trim().to_lowercase();
                    if response != "y" && response != "yes" {
                        cx.editor.set_status("Paste cancelled");
                        return;
                    }

                    perform_copy_with_prompt(cx, &src_path, &dest_path, is_dir, &root);
                },
            );

            EventResult::Consumed(Some(Box::new(move |compositor: &mut Compositor, _cx: &mut Context| {
                compositor.push(Box::new(prompt));
            })))
        } else {
            perform_copy_no_prompt(ctx, &src_path, &dest_path, is_dir, &root);
            EventResult::Consumed(None)
        }
    }
}

impl Component for FileExplorer {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => *event,
            _ => return self.picker.handle_event(event, ctx),
        };

        // Handle file management keys
        match key_event {
            key!('a') => self.handle_create(ctx),
            key!('d') => self.handle_delete(ctx),
            key!('y') => self.handle_yank(ctx),
            key!('p') => self.handle_paste(ctx),
            // Delegate all other keys to the picker
            _ => self.picker.handle_event(event, ctx),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, ctx: &mut Context) {
        self.picker.render(area, surface, ctx);
    }

    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind) {
        self.picker.cursor(area, ctx)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.picker.required_size(viewport)
    }

    fn id(&self) -> Option<&'static str> {
        Some(ID)
    }
}

/// Schedule a refresh of the file explorer after an operation
/// Note: The Prompt auto-closes after Validate, so we only need to pop the file explorer
fn schedule_refresh_with_prompt(cx: &mut compositor::Context, root: PathBuf) {
    let callback = Box::pin(async move {
        let call: JobCallback =
            JobCallback::EditorCompositor(Box::new(move |editor, compositor| {
                // Pop the current file explorer (wrapped in overlay)
                // Note: Prompt already closed itself after Validate
                compositor.pop();
                // Push a fresh file explorer
                if let Ok(explorer) = FileExplorer::new(root, editor) {
                    compositor.push(Box::new(overlay::overlaid(explorer)));
                }
            }));
        Ok(call)
    });
    cx.jobs.callback(callback);
}

/// Schedule a refresh of the file explorer (no prompt to pop)
fn schedule_refresh_no_prompt(cx: &mut Context, root: PathBuf) {
    let callback = Box::pin(async move {
        let call: JobCallback =
            JobCallback::EditorCompositor(Box::new(move |editor, compositor| {
                // Pop the current file explorer (wrapped in overlay)
                compositor.pop();
                // Push a fresh file explorer
                if let Ok(explorer) = FileExplorer::new(root, editor) {
                    compositor.push(Box::new(overlay::overlaid(explorer)));
                }
            }));
        Ok(call)
    });
    cx.jobs.callback(callback);
}

/// Perform copy operation and refresh (called from prompt)
fn perform_copy_with_prompt(
    cx: &mut compositor::Context,
    src: &Path,
    dest: &Path,
    is_dir: bool,
    root: &Path,
) {
    let result = if is_dir {
        copy_dir_recursive(src, dest)
    } else {
        std::fs::copy(src, dest).map(|_| ())
    };

    match result {
        Ok(_) => {
            cx.editor.set_status(format!(
                "Copied: {} -> {}",
                src.display(),
                dest.display()
            ));
            schedule_refresh_with_prompt(cx, root.to_path_buf());
        }
        Err(e) => {
            cx.editor.set_error(format!("Failed to copy: {}", e));
        }
    }
}

/// Perform copy operation and refresh (no prompt)
fn perform_copy_no_prompt(
    cx: &mut Context,
    src: &Path,
    dest: &Path,
    is_dir: bool,
    root: &Path,
) {
    let result = if is_dir {
        copy_dir_recursive(src, dest)
    } else {
        std::fs::copy(src, dest).map(|_| ())
    };

    match result {
        Ok(_) => {
            cx.editor.set_status(format!(
                "Copied: {} -> {}",
                src.display(),
                dest.display()
            ));
            schedule_refresh_no_prompt(cx, root.to_path_buf());
        }
        Err(e) => {
            cx.editor.set_error(format!("Failed to copy: {}", e));
        }
    }
}

/// Create the underlying picker for the file explorer
fn create_picker(
    root: PathBuf,
    editor: &Editor,
) -> Result<Picker<(PathBuf, bool), (PathBuf, Style)>, std::io::Error> {
    let directory_style = editor.theme.get("ui.text.directory");
    let directory_content = directory_content(&root, editor)?;

    let columns = [PickerColumn::new(
        "path",
        |(path, is_dir): &(PathBuf, bool), (root, directory_style): &(PathBuf, Style)| {
            let name = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
            if *is_dir {
                tui::text::Span::styled(format!("{}/", name), *directory_style).into()
            } else {
                name.into()
            }
        },
    )];

    let picker = Picker::new(
        columns,
        0,
        directory_content,
        (root.clone(), directory_style),
        move |cx, (path, is_dir): &(PathBuf, bool), action| {
            if *is_dir {
                let new_root = helix_stdx::path::normalize(path);
                let callback = Box::pin(async move {
                    let call: JobCallback =
                        JobCallback::EditorCompositor(Box::new(move |editor, compositor| {
                            if let Ok(explorer) = FileExplorer::new(new_root, editor) {
                                compositor.push(Box::new(overlay::overlaid(explorer)));
                            }
                        }));
                    Ok(call)
                });
                cx.jobs.callback(callback);
            } else if let Err(e) = cx.editor.open(path, action) {
                let err = if let Some(err) = std::error::Error::source(&e) {
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

/// Get directory contents for the picker
pub fn directory_content(root: &Path, editor: &Editor) -> Result<Vec<(PathBuf, bool)>, std::io::Error> {
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

pub fn get_child_if_single_dir(path: &Path) -> Option<PathBuf> {
    let mut entries = path.read_dir().ok()?;
    let entry = entries.next()?.ok()?;
    if entries.next().is_none() && entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
        Some(entry.path())
    } else {
        None
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
        .expect("Invalid type definition");
    type_builder.negate("all");
    type_builder
        .build()
        .expect("failed to build excluded_types")
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_copy_dir_recursive() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create source structure: src/a.txt, src/subdir/b.txt
        fs::create_dir_all(src.join("subdir")).unwrap();
        File::create(src.join("a.txt"))
            .unwrap()
            .write_all(b"file a")
            .unwrap();
        File::create(src.join("subdir/b.txt"))
            .unwrap()
            .write_all(b"file b")
            .unwrap();

        // Copy
        copy_dir_recursive(&src, &dest).unwrap();

        // Verify
        assert!(dest.join("a.txt").exists());
        assert!(dest.join("subdir/b.txt").exists());
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "file a");
        assert_eq!(
            fs::read_to_string(dest.join("subdir/b.txt")).unwrap(),
            "file b"
        );
    }

    #[test]
    fn test_copy_dir_recursive_empty_dir() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        fs::create_dir(&src).unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        assert!(dest.exists());
        assert!(dest.is_dir());
    }

    #[test]
    fn test_clipboard_operations() {
        // Clear clipboard first
        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = None;
        }

        // Verify empty
        let entry = get_clipboard().lock().ok().and_then(|g| g.clone());
        assert!(entry.is_none());

        // Set clipboard
        let test_path = PathBuf::from("/test/path.txt");
        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = Some(ClipboardEntry {
                path: test_path.clone(),
            });
        }

        // Verify set
        let entry = get_clipboard().lock().ok().and_then(|g| g.clone());
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().path, test_path);
    }

    #[test]
    fn test_get_child_if_single_dir() {
        let temp = tempfile::tempdir().unwrap();

        // Empty dir - no single child
        assert_eq!(get_child_if_single_dir(temp.path()), None);

        // Single directory child
        let child_dir = temp.path().join("only_child");
        fs::create_dir(&child_dir).unwrap();
        assert_eq!(get_child_if_single_dir(temp.path()), Some(child_dir.clone()));

        // Add a file - no longer single dir child
        File::create(temp.path().join("file.txt")).unwrap();
        assert_eq!(get_child_if_single_dir(temp.path()), None);
    }

    #[test]
    fn test_get_child_if_single_dir_with_file_only() {
        let temp = tempfile::tempdir().unwrap();

        // Single file (not dir) - should return None
        File::create(temp.path().join("file.txt")).unwrap();
        assert_eq!(get_child_if_single_dir(temp.path()), None);
    }

    #[test]
    fn test_create_nested_path() {
        let temp = tempfile::tempdir().unwrap();
        let nested_path = temp.path().join("a/b/c/file.txt");

        // Create parent directories
        if let Some(parent) = nested_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        File::create(&nested_path).unwrap();

        assert!(nested_path.exists());
        assert!(temp.path().join("a/b/c").is_dir());
    }

    #[test]
    fn test_create_directory_with_trailing_slash() {
        let temp = tempfile::tempdir().unwrap();
        let input = "newdir/";
        let path = temp.path().join(input);

        // Simulate the create logic for directories
        let dir_path = path.with_file_name(
            path.file_name()
                .map(|n| {
                    n.to_string_lossy()
                        .trim_end_matches('/')
                        .trim_end_matches(std::path::MAIN_SEPARATOR)
                        .to_string()
                })
                .unwrap_or_default(),
        );

        fs::create_dir_all(&dir_path).unwrap();

        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }
}
