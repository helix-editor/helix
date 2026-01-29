//! File explorer with file management capabilities.
//!
//! Provides create, delete, rename, move, yank (copy), cut, and paste operations with live feedback.

use std::error::Error;
use std::path::{Path, PathBuf};

use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Rect},
    theme::Style,
    Editor,
};
use tui::buffer::Buffer as Surface;

use crate::{
    alt,
    compositor::{self, Component, Compositor, Context, Event, EventResult},
    job::Callback as JobCallback,
    ui::{self, overlay, Picker, PickerColumn, Prompt, PromptEvent},
};

pub const ID: &str = "file-explorer";

/// Type alias for the file explorer picker to reduce type complexity
type FileExplorerPicker = Picker<(PathBuf, bool), (PathBuf, Style)>;

/// Type of clipboard operation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardOperation {
    Copy,
    Cut,
}

/// Clipboard entry for copy/cut/paste operations
#[derive(Clone)]
pub struct ClipboardEntry {
    pub path: PathBuf,
    pub operation: ClipboardOperation,
}

/// Global clipboard for file operations (shared across file explorer instances)
static CLIPBOARD: std::sync::OnceLock<std::sync::Mutex<Option<ClipboardEntry>>> =
    std::sync::OnceLock::new();

fn get_clipboard() -> &'static std::sync::Mutex<Option<ClipboardEntry>> {
    CLIPBOARD.get_or_init(|| std::sync::Mutex::new(None))
}

/// Check if an IO error indicates a cross-device link error.
fn is_cross_device_error(e: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        e.raw_os_error() == Some(libc::EXDEV)
    }
    #[cfg(windows)]
    {
        e.raw_os_error() == Some(17) // ERROR_NOT_SAME_DEVICE
    }
    #[cfg(not(any(unix, windows)))]
    {
        // Conservative fallback: assume cross-device for any OS error
        e.raw_os_error().is_some()
    }
}

/// File explorer with file management capabilities
pub struct FileExplorer {
    picker: FileExplorerPicker,
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

                let is_dir = input.ends_with('/') || input.ends_with(std::path::MAIN_SEPARATOR);
                let trimmed_input = input.trim_end_matches(['/', std::path::MAIN_SEPARATOR]);

                // Reject empty names (e.g., input was just "/" or "\")
                if trimmed_input.is_empty() {
                    cx.editor.set_error("Filename cannot be empty");
                    return;
                }

                // Reject filenames with leading/trailing whitespace in any component
                if trimmed_input
                    .split(['/', std::path::MAIN_SEPARATOR])
                    .any(|c| c != c.trim())
                {
                    cx.editor
                        .set_error("Filename cannot have leading or trailing whitespace");
                    return;
                }

                // Reject ".." path components to prevent directory traversal
                if trimmed_input
                    .split(['/', std::path::MAIN_SEPARATOR])
                    .any(|c| c == "..")
                {
                    cx.editor
                        .set_error("Cannot create files outside the current directory");
                    return;
                }

                let path = current_dir.join(trimmed_input);

                // Validate path is within current directory (prevent path traversal)
                let canonical_current = match current_dir.canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        cx.editor
                            .set_error(format!("Failed to resolve current directory: {}", e));
                        return;
                    }
                };

                // For paths that can be fully resolved, verify they stay within current_dir.
                // This handles edge cases like symlinks that could escape the directory.
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        if let Ok(canonical_parent) = parent.canonicalize() {
                            if !canonical_parent.starts_with(&canonical_current) {
                                cx.editor
                                    .set_error("Cannot create files outside the current directory");
                                return;
                            }
                        }
                    }
                }

                let result = if is_dir {
                    // Create directory
                    std::fs::create_dir_all(&path).map(|_| path.clone())
                } else {
                    // Create file (and parent directories if needed)
                    if let Some(parent) = path.parent() {
                        if !parent.exists() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                cx.editor
                                    .set_error(format!("Failed to create directories: {}", e));
                                return;
                            }
                        }
                    }
                    std::fs::File::create(&path).map(|_| path.clone())
                };

                match result {
                    Ok(created_path) => {
                        cx.editor
                            .set_status(format!("Created: {}", created_path.display()));
                        schedule_refresh(cx, root.clone());
                    }
                    Err(e) => {
                        cx.editor.set_error(format!("Failed to create: {}", e));
                    }
                }
            },
        );

        EventResult::Consumed(Some(Box::new(
            move |compositor: &mut Compositor, _cx: &mut Context| {
                compositor.push(Box::new(prompt));
            },
        )))
    }

    /// Handle 'd' key - delete file or directory
    fn handle_delete(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow deleting ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor
                .set_error("Cannot delete parent directory reference");
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
                        schedule_refresh(cx, root.clone());
                    }
                    Err(e) => {
                        cx.editor.set_error(format!("Failed to delete: {}", e));
                    }
                }
            },
        );

        EventResult::Consumed(Some(Box::new(
            move |compositor: &mut Compositor, _cx: &mut Context| {
                compositor.push(Box::new(prompt));
            },
        )))
    }

    /// Handle 'y' key - yank (copy) file/directory to clipboard
    fn handle_yank(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, _is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow yanking ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor
                .set_error("Cannot yank parent directory reference");
            return EventResult::Consumed(None);
        }

        match get_clipboard().lock() {
            Ok(mut clipboard) => {
                *clipboard = Some(ClipboardEntry {
                    path: path.clone(),
                    operation: ClipboardOperation::Copy,
                });
                ctx.editor.set_status(format!("Yanked: {}", path.display()));
            }
            Err(_) => {
                ctx.editor
                    .set_error("Failed to access clipboard for yank operation");
            }
        }
        EventResult::Consumed(None)
    }

    /// Handle 'x' key - cut file/directory to clipboard
    fn handle_cut(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, _is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow cutting ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor
                .set_error("Cannot cut parent directory reference");
            return EventResult::Consumed(None);
        }

        match get_clipboard().lock() {
            Ok(mut clipboard) => {
                *clipboard = Some(ClipboardEntry {
                    path: path.clone(),
                    operation: ClipboardOperation::Cut,
                });
                ctx.editor.set_status(format!("Cut: {}", path.display()));
            }
            Err(_) => {
                ctx.editor
                    .set_error("Failed to access clipboard for cut operation");
            }
        }
        EventResult::Consumed(None)
    }

    /// Handle 'r' key - rename file/directory
    fn handle_rename(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, _is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow renaming ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor
                .set_error("Cannot rename parent directory reference");
            return EventResult::Consumed(None);
        }

        let old_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let root = self.root.clone();
        let parent = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.clone());
        let old_name_for_prompt = old_name.clone();

        let prompt = Prompt::new(
            "rename: ".into(),
            None,
            ui::completers::none,
            move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                if event != PromptEvent::Validate {
                    return;
                }

                if input.is_empty() {
                    return;
                }

                // Reject path separators to prevent path traversal attacks.
                // Rename should only change the filename within the same directory.
                if input.contains('/') || input.contains(std::path::MAIN_SEPARATOR) {
                    cx.editor
                        .set_error("Invalid filename: cannot contain path separators");
                    return;
                }

                // Reject "." and ".." as filenames (they're special directory references)
                if input == "." || input == ".." {
                    cx.editor
                        .set_error("Invalid filename: '.' and '..' are reserved");
                    return;
                }

                // Reject filenames with leading/trailing whitespace
                if input != input.trim() {
                    cx.editor
                        .set_error("Filename cannot have leading or trailing whitespace");
                    return;
                }

                let new_path = parent.join(input);

                // Check if destination exists
                if new_path.exists() {
                    cx.editor.set_error(format!("'{}' already exists", input));
                    return;
                }

                match std::fs::rename(&path, &new_path) {
                    Ok(_) => {
                        cx.editor
                            .set_status(format!("Renamed: {} -> {}", old_name, input));
                        schedule_refresh(cx, root.clone());
                    }
                    Err(e) => {
                        cx.editor.set_error(format!("Failed to rename: {}", e));
                    }
                }
            },
        )
        .with_line(old_name_for_prompt, ctx.editor);

        EventResult::Consumed(Some(Box::new(
            move |compositor: &mut Compositor, _cx: &mut Context| {
                compositor.push(Box::new(prompt));
            },
        )))
    }

    /// Handle 'm' key - move file/directory to new location
    fn handle_move(&mut self, ctx: &mut Context) -> EventResult {
        let Some((path, is_dir)) = self.selected_path().cloned() else {
            ctx.editor.set_error("No file selected");
            return EventResult::Consumed(None);
        };

        // Don't allow moving ".."
        if path.file_name().map(|n| n == "..").unwrap_or(false) {
            ctx.editor
                .set_error("Cannot move parent directory reference");
            return EventResult::Consumed(None);
        }

        let root = self.root.clone();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let prompt = Prompt::new(
            "move to: ".into(),
            None,
            ui::completers::directory,
            move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
                if event != PromptEvent::Validate {
                    return;
                }

                if input.is_empty() {
                    return;
                }

                // Expand ~ and resolve path
                let dest_dir = helix_stdx::path::expand_tilde(Path::new(input));
                let dest_path = if dest_dir.is_dir() {
                    dest_dir.join(&file_name)
                } else {
                    dest_dir.to_path_buf()
                };

                // Check if destination exists before creating directories
                if dest_path.exists() {
                    cx.editor
                        .set_error(format!("'{}' already exists", dest_path.display()));
                    return;
                }

                // Create parent directories if needed
                if let Some(parent) = dest_path.parent() {
                    if !parent.exists() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            cx.editor
                                .set_error(format!("Failed to create directories: {}", e));
                            return;
                        }
                    }
                }

                // Prevent moving a directory into itself or its subdirectories
                if is_dir {
                    if let Ok(src_canonical) = path.canonicalize() {
                        if let Some(parent) = dest_path.parent() {
                            if let Ok(dest_canonical) = parent.canonicalize() {
                                if dest_canonical.starts_with(&src_canonical) {
                                    cx.editor.set_error("Cannot move a directory into itself");
                                    return;
                                }
                            }
                        }
                    }
                }

                // Check if destination is outside the file explorer root.
                // This helps users notice moves to unintended locations (e.g., ~/Downloads vs ./Downloads).
                let is_outside_root = if let Ok(canonical_root) = root.canonicalize() {
                    if let Some(parent) = dest_path.parent() {
                        if let Ok(canonical_dest_parent) = parent.canonicalize() {
                            !canonical_dest_parent.starts_with(&canonical_root)
                        } else {
                            // Can't canonicalize - destination parent doesn't exist yet, check if it will be outside
                            !parent.starts_with(&root)
                        }
                    } else {
                        true // No parent means root-level, which is outside
                    }
                } else {
                    false // Can't canonicalize root, skip the check
                };

                match std::fs::rename(&path, &dest_path) {
                    Ok(_) => {
                        let verb = if is_dir { "Moved directory" } else { "Moved" };
                        // Include "(outside file explorer)" suffix to warn user of external move
                        let suffix = if is_outside_root {
                            " (outside file explorer)"
                        } else {
                            ""
                        };
                        cx.editor.set_status(format!(
                            "{}: {} -> {}{}",
                            verb,
                            path.display(),
                            dest_path.display(),
                            suffix
                        ));
                        schedule_refresh(cx, root.clone());
                    }
                    Err(e) => {
                        // Cross-device move: try copy + delete
                        if is_cross_device_error(&e) {
                            if is_dir {
                                match copy_dir_recursive(&path, &dest_path) {
                                    Ok(_) => {
                                        if let Err(del_err) = std::fs::remove_dir_all(&path) {
                                            cx.editor.set_error(format!(
                                                "Copied but failed to remove source: {}",
                                                del_err
                                            ));
                                            return;
                                        }
                                        let suffix = if is_outside_root {
                                            " (outside file explorer)"
                                        } else {
                                            ""
                                        };
                                        cx.editor.set_status(format!(
                                            "Moved directory: {} -> {}{}",
                                            path.display(),
                                            dest_path.display(),
                                            suffix
                                        ));
                                        schedule_refresh(cx, root.clone());
                                    }
                                    Err(copy_err) => {
                                        cx.editor
                                            .set_error(format!("Failed to move: {}", copy_err));
                                    }
                                }
                            } else {
                                match std::fs::copy(&path, &dest_path) {
                                    Ok(_) => {
                                        if let Err(del_err) = std::fs::remove_file(&path) {
                                            cx.editor.set_error(format!(
                                                "Copied but failed to remove source: {}",
                                                del_err
                                            ));
                                            return;
                                        }
                                        let suffix = if is_outside_root {
                                            " (outside file explorer)"
                                        } else {
                                            ""
                                        };
                                        cx.editor.set_status(format!(
                                            "Moved: {} -> {}{}",
                                            path.display(),
                                            dest_path.display(),
                                            suffix
                                        ));
                                        schedule_refresh(cx, root.clone());
                                    }
                                    Err(copy_err) => {
                                        cx.editor
                                            .set_error(format!("Failed to move: {}", copy_err));
                                    }
                                }
                            }
                        } else {
                            cx.editor.set_error(format!("Failed to move: {}", e));
                        }
                    }
                }
            },
        );

        EventResult::Consumed(Some(Box::new(
            move |compositor: &mut Compositor, _cx: &mut Context| {
                compositor.push(Box::new(prompt));
            },
        )))
    }

    /// Handle 'p' key - paste from clipboard
    fn handle_paste(&mut self, ctx: &mut Context) -> EventResult {
        let clipboard_entry = get_clipboard().lock().ok().and_then(|guard| guard.clone());

        let Some(entry) = clipboard_entry else {
            ctx.editor.set_error("Clipboard is empty");
            return EventResult::Consumed(None);
        };

        let src_path = entry.path.clone();

        // Verify source still exists (it may have been deleted since yank/cut)
        if !src_path.exists() {
            ctx.editor
                .set_error(format!("Source no longer exists: {}", src_path.display()));
            // Clear invalid clipboard entry
            if let Ok(mut clipboard) = get_clipboard().lock() {
                *clipboard = None;
            }
            return EventResult::Consumed(None);
        }

        let dest_dir = self.current_directory();
        let file_name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        let dest_path = dest_dir.join(&file_name);
        let root = self.root.clone();
        let is_dir = src_path.is_dir();
        let operation = entry.operation;

        // Prevent pasting a directory into itself or its subdirectories
        if is_dir {
            if let Ok(src_canonical) = src_path.canonicalize() {
                if let Ok(dest_canonical) = dest_dir.canonicalize() {
                    if dest_canonical.starts_with(&src_canonical) {
                        ctx.editor.set_error("Cannot paste a directory into itself");
                        return EventResult::Consumed(None);
                    }
                }
            }
        }

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

                    // Re-check destination existence to reduce TOCTOU risk between prompt and confirmation.
                    // Note: A small race window still exists between this check and the actual operation,
                    // but this significantly reduces the risk of unexpected behavior.
                    if dest_path.exists() {
                        // Destination still exists: proceed with overwrite as confirmed.
                        perform_paste(cx, &src_path, &dest_path, is_dir, operation, &root, true);
                    } else {
                        // Destination disappeared: fall back to non-overwrite behavior.
                        perform_paste(cx, &src_path, &dest_path, is_dir, operation, &root, false);
                    }
                },
            );

            EventResult::Consumed(Some(Box::new(
                move |compositor: &mut Compositor, _cx: &mut Context| {
                    compositor.push(Box::new(prompt));
                },
            )))
        } else {
            // Re-check just before performing the paste to reduce TOCTOU risk.
            // Note: A small race window still exists between this check and the actual
            // file operation in perform_paste. This is a known limitation; fully eliminating
            // TOCTOU would require exclusive file creation flags at the filesystem level.
            if dest_path.exists() {
                ctx.editor
                    .set_error("Destination already exists; paste cancelled");
                EventResult::Consumed(None)
            } else {
                perform_paste(ctx, &src_path, &dest_path, is_dir, operation, &root, false);
                EventResult::Consumed(None)
            }
        }
    }
}

impl Component for FileExplorer {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => *event,
            _ => return self.picker.handle_event(event, ctx),
        };

        // Handle file management keys (Alt/Option + key to not interfere with search)
        match key_event {
            alt!('a') => self.handle_create(ctx),
            alt!('d') => self.handle_delete(ctx),
            alt!('r') => self.handle_rename(ctx),
            alt!('m') => self.handle_move(ctx),
            alt!('y') => self.handle_yank(ctx),
            alt!('x') => self.handle_cut(ctx),
            alt!('p') => self.handle_paste(ctx),
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
fn schedule_refresh(cx: &mut Context, root: PathBuf) {
    let callback = Box::pin(async move {
        let call: JobCallback =
            JobCallback::EditorCompositor(Box::new(move |editor, compositor| {
                // Remove the file explorer by ID instead of blindly popping.
                // This prevents accidentally removing unrelated overlays (like prompts)
                // that may have been opened after the refresh was scheduled.
                compositor.remove(ID);
                // Push a fresh file explorer
                if let Ok(explorer) = FileExplorer::new(root, editor) {
                    compositor.push(Box::new(overlay::overlaid(explorer)));
                }
            }));
        Ok(call)
    });
    cx.jobs.callback(callback);
}

/// Perform paste operation and refresh
///
/// If `overwrite` is true, removes the existing destination before pasting.
fn perform_paste(
    cx: &mut Context,
    src: &Path,
    dest: &Path,
    is_dir: bool,
    operation: ClipboardOperation,
    root: &Path,
    overwrite: bool,
) {
    // Remove existing destination if overwriting (user confirmed)
    if overwrite && dest.exists() {
        let remove_result = if dest.is_dir() {
            std::fs::remove_dir_all(dest)
        } else {
            std::fs::remove_file(dest)
        };
        if let Err(e) = remove_result {
            cx.editor
                .set_error(format!("Failed to remove existing destination: {}", e));
            return;
        }
    }

    let (result, verb) = match operation {
        ClipboardOperation::Copy => {
            let res = if is_dir {
                copy_dir_recursive(src, dest)
            } else {
                std::fs::copy(src, dest).map(|_| ())
            };
            (res, "Copied")
        }
        ClipboardOperation::Cut => {
            // Try rename first (fast path for same filesystem)
            let res = std::fs::rename(src, dest).or_else(|e| {
                // Cross-device: copy then delete
                if is_cross_device_error(&e) {
                    if is_dir {
                        copy_dir_recursive(src, dest)?;
                        std::fs::remove_dir_all(src)
                    } else {
                        std::fs::copy(src, dest)?;
                        std::fs::remove_file(src)
                    }
                } else {
                    Err(e)
                }
            });
            // Clear clipboard after successful cut
            if res.is_ok() {
                if let Ok(mut clipboard) = get_clipboard().lock() {
                    *clipboard = None;
                }
            }
            (res, "Moved")
        }
    };

    match result {
        Ok(_) => {
            cx.editor
                .set_status(format!("{}: {} -> {}", verb, src.display(), dest.display()));
            schedule_refresh(cx, root.to_path_buf());
        }
        Err(e) => {
            cx.editor.set_error(format!("Failed to paste: {}", e));
        }
    }
}

/// Create the underlying picker for the file explorer
fn create_picker(root: PathBuf, editor: &Editor) -> Result<FileExplorerPicker, std::io::Error> {
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

/// Get directory contents for the picker
pub fn directory_content(
    root: &Path,
    editor: &Editor,
) -> Result<Vec<(PathBuf, bool)>, std::io::Error> {
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
        .types(super::get_excluded_types())
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

/// Recursively copy a directory, preserving permissions and skipping symlinks
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    let dest_existed = dest.exists();
    std::fs::create_dir_all(dest)?;

    // Preserve directory permissions if we created the directory
    if !dest_existed {
        if let Ok(src_metadata) = std::fs::metadata(src) {
            // Ignore permission copy errors to avoid changing existing behavior
            let _ = std::fs::set_permissions(dest, src_metadata.permissions());
        }
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        // Use symlink_metadata to inspect the entry itself without following symlinks
        // This is equivalent to lstat() on Unix and avoids symlink loops
        let file_type = std::fs::symlink_metadata(&src_path)?.file_type();

        if file_type.is_symlink() {
            // Skip symlinks to avoid following symlink loops
            continue;
        } else if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dest_path)?;
        }
        // Skip other types (sockets, etc.)
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
    fn test_clipboard_copy_operation() {
        // Clear clipboard first
        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = None;
        }

        // Verify empty
        let entry = get_clipboard().lock().ok().and_then(|g| g.clone());
        assert!(entry.is_none());

        // Set clipboard with Copy operation
        let test_path = PathBuf::from("/test/path.txt");
        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = Some(ClipboardEntry {
                path: test_path.clone(),
                operation: ClipboardOperation::Copy,
            });
        }

        // Verify set
        let entry = get_clipboard().lock().ok().and_then(|g| g.clone());
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.path, test_path);
        assert_eq!(entry.operation, ClipboardOperation::Copy);
    }

    #[test]
    fn test_clipboard_cut_operation() {
        // Set clipboard with Cut operation
        let test_path = PathBuf::from("/test/cut_path.txt");
        if let Ok(mut clipboard) = get_clipboard().lock() {
            *clipboard = Some(ClipboardEntry {
                path: test_path.clone(),
                operation: ClipboardOperation::Cut,
            });
        }

        // Verify set
        let entry = get_clipboard().lock().ok().and_then(|g| g.clone());
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.path, test_path);
        assert_eq!(entry.operation, ClipboardOperation::Cut);
    }

    #[test]
    fn test_get_child_if_single_dir() {
        let temp = tempfile::tempdir().unwrap();

        // Empty dir - no single child
        assert_eq!(get_child_if_single_dir(temp.path()), None);

        // Single directory child
        let child_dir = temp.path().join("only_child");
        fs::create_dir(&child_dir).unwrap();
        assert_eq!(
            get_child_if_single_dir(temp.path()),
            Some(child_dir.clone())
        );

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

    #[cfg(unix)]
    #[test]
    fn test_copy_dir_recursive_skips_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create source with a symlink
        fs::create_dir_all(&src).unwrap();
        File::create(src.join("real_file.txt"))
            .unwrap()
            .write_all(b"real content")
            .unwrap();

        // Create a symlink (should be skipped)
        symlink(src.join("real_file.txt"), src.join("link_to_file")).unwrap();

        // Create a directory symlink (should also be skipped)
        let other_dir = temp.path().join("other");
        fs::create_dir(&other_dir).unwrap();
        symlink(&other_dir, src.join("link_to_dir")).unwrap();

        // Copy
        copy_dir_recursive(&src, &dest).unwrap();

        // Verify: real file copied, symlinks skipped
        assert!(dest.join("real_file.txt").exists());
        assert!(!dest.join("link_to_file").exists());
        assert!(!dest.join("link_to_dir").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_dir_recursive_handles_circular_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create source with a circular symlink
        fs::create_dir_all(&src).unwrap();
        File::create(src.join("file.txt"))
            .unwrap()
            .write_all(b"content")
            .unwrap();

        // Create circular symlink: src/loop -> src
        symlink(&src, src.join("loop")).unwrap();

        // Copy should succeed (symlinks are skipped)
        let result = copy_dir_recursive(&src, &dest);
        assert!(result.is_ok());

        // Verify file copied, circular link skipped
        assert!(dest.join("file.txt").exists());
        assert!(!dest.join("loop").exists());
    }

    #[test]
    fn test_copy_dir_recursive_preserves_nested_structure() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create deeply nested structure
        fs::create_dir_all(src.join("a/b/c/d")).unwrap();
        File::create(src.join("a/b/c/d/deep.txt"))
            .unwrap()
            .write_all(b"deep content")
            .unwrap();
        File::create(src.join("a/b/mid.txt"))
            .unwrap()
            .write_all(b"mid content")
            .unwrap();
        File::create(src.join("a/shallow.txt"))
            .unwrap()
            .write_all(b"shallow content")
            .unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        // Verify all levels
        assert!(dest.join("a/b/c/d").is_dir());
        assert_eq!(
            fs::read_to_string(dest.join("a/b/c/d/deep.txt")).unwrap(),
            "deep content"
        );
        assert_eq!(
            fs::read_to_string(dest.join("a/b/mid.txt")).unwrap(),
            "mid content"
        );
        assert_eq!(
            fs::read_to_string(dest.join("a/shallow.txt")).unwrap(),
            "shallow content"
        );
    }

    #[test]
    fn test_is_cross_device_error_with_regular_error() {
        // Regular IO errors should not be detected as cross-device
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        assert!(!is_cross_device_error(&err));

        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        assert!(!is_cross_device_error(&err));
    }

    #[cfg(unix)]
    #[test]
    fn test_is_cross_device_error_unix() {
        // On Unix, EXDEV (18) should be detected as cross-device
        let err = std::io::Error::from_raw_os_error(libc::EXDEV);
        assert!(is_cross_device_error(&err));

        // Other OS errors should not match
        let err = std::io::Error::from_raw_os_error(libc::ENOENT);
        assert!(!is_cross_device_error(&err));
    }

    #[test]
    fn test_path_starts_with_for_self_reference_detection() {
        // Test the logic used to detect moving/pasting into self
        let parent = PathBuf::from("/home/user/documents");
        let child = PathBuf::from("/home/user/documents/subfolder");
        let sibling = PathBuf::from("/home/user/downloads");

        // Child starts with parent (would be invalid move target)
        assert!(child.starts_with(&parent));

        // Sibling does not start with parent (valid move target)
        assert!(!sibling.starts_with(&parent));

        // Parent does not start with child
        assert!(!parent.starts_with(&child));
    }

    #[test]
    fn test_clipboard_operations_are_distinct() {
        assert_ne!(ClipboardOperation::Copy, ClipboardOperation::Cut);

        // Test debug output
        assert_eq!(format!("{:?}", ClipboardOperation::Copy), "Copy");
        assert_eq!(format!("{:?}", ClipboardOperation::Cut), "Cut");
    }

    #[test]
    fn test_clipboard_entry_clone() {
        let entry = ClipboardEntry {
            path: PathBuf::from("/test/path"),
            operation: ClipboardOperation::Copy,
        };

        let cloned = entry.clone();
        assert_eq!(cloned.path, entry.path);
        assert_eq!(cloned.operation, entry.operation);
    }

    #[test]
    fn test_copy_dir_recursive_overwrites_existing_files() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create source
        fs::create_dir_all(&src).unwrap();
        File::create(src.join("file.txt"))
            .unwrap()
            .write_all(b"new content")
            .unwrap();

        // Create destination with existing file
        fs::create_dir_all(&dest).unwrap();
        File::create(dest.join("file.txt"))
            .unwrap()
            .write_all(b"old content")
            .unwrap();

        // Copy should overwrite
        copy_dir_recursive(&src, &dest).unwrap();

        assert_eq!(
            fs::read_to_string(dest.join("file.txt")).unwrap(),
            "new content"
        );
    }

    #[test]
    fn test_copy_dir_recursive_merges_directories() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create source with file_a
        fs::create_dir_all(&src).unwrap();
        File::create(src.join("file_a.txt"))
            .unwrap()
            .write_all(b"content a")
            .unwrap();

        // Create destination with file_b
        fs::create_dir_all(&dest).unwrap();
        File::create(dest.join("file_b.txt"))
            .unwrap()
            .write_all(b"content b")
            .unwrap();

        // Copy should merge (file_b preserved, file_a added)
        copy_dir_recursive(&src, &dest).unwrap();

        assert!(dest.join("file_a.txt").exists());
        assert!(dest.join("file_b.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("file_a.txt")).unwrap(),
            "content a"
        );
        assert_eq!(
            fs::read_to_string(dest.join("file_b.txt")).unwrap(),
            "content b"
        );
    }

    #[test]
    fn test_path_traversal_input_detection() {
        // Test that path traversal sequences can be detected
        let current_dir = PathBuf::from("/home/user/project");

        // Normal input - stays within directory
        let normal_input = "subdir/file.txt";
        let normal_path = current_dir.join(normal_input);
        assert!(normal_path.starts_with(&current_dir));

        // Path traversal attempt - would escape directory
        // Note: This tests the raw join behavior; actual validation uses canonicalize
        let traversal_input = "../../../etc/passwd";
        let traversal_path = current_dir.join(traversal_input);
        // The joined path contains ".." but that's handled by canonicalize in actual code
        assert!(traversal_path.to_string_lossy().contains(".."));
    }

    #[test]
    fn test_trailing_separator_handling() {
        // Test the logic for handling trailing separators in directory creation
        let input_with_slash = "newdir/";
        let input_with_backslash = "newdir\\";
        let input_clean = "newdir";

        let trimmed1 = input_with_slash.trim_end_matches(['/', std::path::MAIN_SEPARATOR]);
        let trimmed2 = input_with_backslash.trim_end_matches(['/', std::path::MAIN_SEPARATOR]);

        assert_eq!(trimmed1, "newdir");
        // On Unix, backslash is not a separator so it won't be trimmed
        #[cfg(unix)]
        assert_eq!(trimmed2, "newdir\\");
        #[cfg(windows)]
        assert_eq!(trimmed2, "newdir");

        assert_eq!(
            input_clean.trim_end_matches(['/', std::path::MAIN_SEPARATOR]),
            "newdir"
        );
    }
}
