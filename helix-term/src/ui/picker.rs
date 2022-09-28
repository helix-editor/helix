use crate::{
    compositor::{Component, Compositor, Context, Event, EventResult},
    ctrl, key, shift,
    ui::{self, EditorView},
};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Borders},
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;
use tui::widgets::Widget;

use std::borrow::Cow;
use std::time::Instant;
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use super::menu::Item;
use crate::ui::{overlay, Prompt, PromptEvent};
use helix_core::{movement::Direction, Position};
use helix_view::{
    editor::Action,
    graphics::{CursorKind, Margin, Modifier, Rect},
    Document, Editor,
};

// based on exa but not sure where to put this
/// More readable aliases for the permission bits exposed by libc.
#[cfg(unix)]
mod modes {
    // The `libc::mode_t` type’s actual type varies, but the value returned
    // from `metadata.permissions().mode()` is always `u32`.
    pub type Mode = u32;

    pub const USER_READ: Mode = libc::S_IRUSR as Mode;
    pub const USER_WRITE: Mode = libc::S_IWUSR as Mode;
    pub const USER_EXECUTE: Mode = libc::S_IXUSR as Mode;

    pub const GROUP_READ: Mode = libc::S_IRGRP as Mode;
    pub const GROUP_WRITE: Mode = libc::S_IWGRP as Mode;
    pub const GROUP_EXECUTE: Mode = libc::S_IXGRP as Mode;

    pub const OTHER_READ: Mode = libc::S_IROTH as Mode;
    pub const OTHER_WRITE: Mode = libc::S_IWOTH as Mode;
    pub const OTHER_EXECUTE: Mode = libc::S_IXOTH as Mode;

    pub const STICKY: Mode = libc::S_ISVTX as Mode;
    pub const SETGID: Mode = libc::S_ISGID as Mode;
    pub const SETUID: Mode = libc::S_ISUID as Mode;
}

#[cfg(not(unix))]
mod modes {}

// based on exa but not sure where to put this
mod fields {
    use super::modes;
    use std::fmt;
    use std::fs::Metadata;
    #[cfg(unix)]
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    /// The file’s base type, which gets displayed in the very first column of the
    /// details output.
    ///
    /// This type is set entirely by the filesystem, rather than relying on a
    /// file’s contents. So “link” is a type, but “image” is just a type of
    /// regular file. (See the `filetype` module for those checks.)
    ///
    /// Its ordering is used when sorting by type.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
    pub enum Type {
        Directory,
        File,
        Link,
        Pipe,
        Socket,
        CharDevice,
        BlockDevice,
        Special,
    }

    #[cfg(unix)]
    pub fn filetype(metadata: &Metadata) -> Type {
        let filetype = metadata.file_type();
        if metadata.is_file() {
            Type::File
        } else if metadata.is_dir() {
            Type::Directory
        } else if filetype.is_fifo() {
            Type::Pipe
        } else if filetype.is_symlink() {
            Type::Link
        } else if filetype.is_char_device() {
            Type::CharDevice
        } else if filetype.is_block_device() {
            Type::BlockDevice
        } else if filetype.is_socket() {
            Type::Socket
        } else {
            Type::Special
        }
    }

    #[cfg(not(unix))]
    pub fn filetype(metadata: &Metadata) -> Type {
        unreachable!()
    }

    impl fmt::Display for Type {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Type::Directory => 'd',
                    Type::File => '.',
                    Type::Link => 'l',
                    Type::Pipe => '|',
                    Type::Socket => 's',
                    Type::CharDevice => 'c',
                    Type::BlockDevice => 'b',
                    Type::Special => '?',
                }
            )
        }
    }

    /// The file’s Unix permission bitfield, with one entry per bit.
    #[derive(Copy, Clone)]
    pub struct Permissions {
        pub user_read: bool,
        pub user_write: bool,
        pub user_execute: bool,

        pub group_read: bool,
        pub group_write: bool,
        pub group_execute: bool,

        pub other_read: bool,
        pub other_write: bool,
        pub other_execute: bool,

        pub sticky: bool,
        pub setgid: bool,
        pub setuid: bool,
    }

    #[cfg(unix)]
    pub fn permissions(metadata: &Metadata) -> Permissions {
        let bits = metadata.mode();
        let has_bit = |bit| bits & bit == bit;
        Permissions {
            user_read: has_bit(modes::USER_READ),
            user_write: has_bit(modes::USER_WRITE),
            user_execute: has_bit(modes::USER_EXECUTE),

            group_read: has_bit(modes::GROUP_READ),
            group_write: has_bit(modes::GROUP_WRITE),
            group_execute: has_bit(modes::GROUP_EXECUTE),

            other_read: has_bit(modes::OTHER_READ),
            other_write: has_bit(modes::OTHER_WRITE),
            other_execute: has_bit(modes::OTHER_EXECUTE),

            sticky: has_bit(modes::STICKY),
            setgid: has_bit(modes::SETGID),
            setuid: has_bit(modes::SETUID),
        }
    }

    #[cfg(not(unix))]
    pub fn permissions(metadata: &Metadata) -> Permissions {
        unreachable!()
    }

    impl fmt::Display for Permissions {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let bit = |bit, char| if bit { char } else { "-" };
            write!(
                f,
                "{}{}{}{}{}{}{}{}{}",
                bit(self.user_read, "r"),
                bit(self.user_write, "w"),
                bit(self.user_execute, "x"),
                bit(self.group_read, "r"),
                bit(self.group_write, "w"),
                bit(self.group_execute, "x"),
                bit(self.other_read, "r"),
                bit(self.other_write, "w"),
                bit(self.other_execute, "x"),
            )
        }
    }

    /// A file’s size, in bytes. This is usually formatted by the `number_prefix`
    /// crate into something human-readable.
    #[derive(Copy, Clone)]
    pub enum Size {
        /// This file has a defined size.
        Some(u64),

        /// This file has no size, or has a size but we aren’t interested in it.
        ///
        /// Under Unix, directory entries that aren’t regular files will still
        /// have a file size. For example, a directory will just contain a list of
        /// its files as its “contents” and will be specially flagged as being a
        /// directory, rather than a file. However, seeing the “file size” of this
        /// data is rarely useful — I can’t think of a time when I’ve seen it and
        /// learnt something. So we discard it and just output “-” instead.
        ///
        /// See this answer for more: http://unix.stackexchange.com/a/68266
        None,

        /// This file is a block or character device, so instead of a size, print
        /// out the file’s major and minor device IDs.
        ///
        /// This is what ls does as well. Without it, the devices will just have
        /// file sizes of zero.
        DeviceIDs(DeviceIDs),
    }

    /// The major and minor device IDs that gets displayed for device files.
    ///
    /// You can see what these device numbers mean:
    /// - <http://www.lanana.org/docs/device-list/>
    /// - <http://www.lanana.org/docs/device-list/devices-2.6+.txt>
    #[derive(Copy, Clone)]
    pub struct DeviceIDs {
        pub major: u8,
        pub minor: u8,
    }

    /// This file’s size, if it’s a regular file.
    ///
    /// For directories, no size is given. Although they do have a size on
    /// some filesystems, I’ve never looked at one of those numbers and gained
    /// any information from it. So it’s going to be hidden instead.
    ///
    /// Block and character devices return their device IDs, because they
    /// usually just have a file size of zero.
    #[cfg(unix)]
    pub fn size(metadata: &Metadata) -> Size {
        let filetype = metadata.file_type();
        if metadata.is_dir() {
            Size::None
        } else if filetype.is_char_device() || filetype.is_block_device() {
            let device_ids = metadata.rdev().to_be_bytes();

            // In C-land, getting the major and minor device IDs is done with
            // preprocessor macros called `major` and `minor` that depend on
            // the size of `dev_t`, but we just take the second-to-last and
            // last bytes.
            Size::DeviceIDs(DeviceIDs {
                major: device_ids[6],
                minor: device_ids[7],
            })
        } else {
            Size::Some(metadata.len())
        }
    }

    #[cfg(not(unix))]
    pub fn size(metadata: &Metadata) -> Size {
        unreachable!()
    }

    impl fmt::Display for Size {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use number_prefix::NumberPrefix;
            match self {
                Size::Some(s) => match NumberPrefix::decimal(*s as f64) {
                    NumberPrefix::Standalone(n) => write!(f, "{}", n),
                    NumberPrefix::Prefixed(p, n) => write!(f, "{:.1}{}", n, p),
                },
                Size::None => write!(f, "-"),
                Size::DeviceIDs(DeviceIDs { major, minor }) => {
                    write!(f, "{},{}", major, minor)
                }
            }
        }
    }
}

pub const MIN_AREA_WIDTH_FOR_PREVIEW: u16 = 72;
/// Biggest file size to preview in bytes
pub const MAX_FILE_SIZE_FOR_PREVIEW: u64 = 10 * 1024 * 1024;

pub enum CachedPreview {
    Document(Box<Document>),
    Binary,
    LargeFile,
    Directory(Vec<String>),
    NotFound,
}

// We don't store this enum in the cache so as to avoid lifetime constraints
// from borrowing a document already opened in the editor.
pub enum Preview<'picker, 'editor> {
    Cached(&'picker CachedPreview),
    EditorDocument(&'editor Document),
}

impl Preview<'_, '_> {
    fn document(&self) -> Option<&Document> {
        match self {
            Preview::EditorDocument(doc) => Some(doc),
            Preview::Cached(CachedPreview::Document(doc)) => Some(doc),
            _ => None,
        }
    }

    fn directory(&self) -> Option<&Vec<String>> {
        match self {
            Preview::Cached(CachedPreview::Directory(preview)) => Some(preview),
            _ => None,
        }
    }

    /// Alternate text to show for the preview.
    fn placeholder(&self) -> &str {
        match *self {
            Self::EditorDocument(_) => "<File preview>",
            Self::Cached(preview) => match preview {
                CachedPreview::Document(_) => "<File preview>",
                CachedPreview::Binary => "<Binary file>",
                CachedPreview::LargeFile => "<File too large to preview>",
                CachedPreview::Directory(_) => "<Directory>",
                CachedPreview::NotFound => "<File not found>",
            },
        }
    }
}

/// File path and range of lines (used to align and highlight lines)
pub type FileLocation = (PathBuf, Option<(usize, usize)>);

// Specialized version of [`FilePicker`] with some custom support to allow directory navigation.
pub struct FindFilePicker {
    picker: FilePicker<PathBuf>,
    dir: PathBuf,
}

impl FindFilePicker {
    pub fn new(dir: PathBuf) -> FindFilePicker {
        // switch to Result::flatten later
        let files: Vec<_> = match fs::read_dir(&dir) {
            Ok(dir) => dir
                .flat_map(|entry| entry.map(|entry| entry.path()))
                .collect(),
            Err(_) => Vec::new(),
        };
        let dir1 = dir.clone();
        let mut picker = FilePicker::new(
            files,
            dir1,
            // TODO: add format_fn
            // TODO: prevent this from running within score function that skews
            // the score, and only calculate it once during initialization
            // move |path| {
            //     let suffix = if path.is_dir() { "/" } else { "" };
            //     let metadata = path.symlink_metadata().unwrap();
            //     let path = path.strip_prefix(&dir1).unwrap_or(path).to_string_lossy();
            //     if cfg!(unix) {
            //         let filetype = fields::filetype(&metadata);
            //         let permissions = fields::permissions(&metadata);
            //         let size = format!("{}", fields::size(&metadata));
            //         Cow::Owned(format!(
            //             "{:<22} {}{} {:>6}",
            //             path + suffix, // TODO this should check for size and handle truncation
            //             filetype,
            //             permissions,
            //             size,
            //             // TODO add absolute/relative time? may need to handle truncation
            //         ))
            //     } else {
            //         path + suffix
            //     }
            // },
            |_cx, _path, _action| {}, // we use custom callback_fn
            |_editor, path| Some((path.clone(), None)),
        );
        // TODO: truncate prompt dir if prompt area too small and current dir too long
        let current_dir = std::env::current_dir().expect("couldn't determine current directory");
        let dir1 = dir.clone();
        let prompt = dir1
            .strip_prefix(current_dir)
            .unwrap_or(&dir1)
            .to_string_lossy()
            .into_owned();
        let prompt = match prompt.as_str() {
            "/" => "/".to_owned(),
            "" => "./".to_owned(),
            _ => prompt + "/",
        };
        *picker.picker.prompt.prompt_mut() = Cow::Owned(prompt);
        picker.picker.prompt.prompt_style_fn = Box::new(|theme| Some(theme.get("blue")));
        FindFilePicker { picker, dir }
    }
}

impl Component for FindFilePicker {
    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        self.picker.render(area, surface, cx);
    }

    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let key = match event {
            Event::Key(key) => *key,
            // TODO we may want to handle mouse event
            Event::Mouse(_) => return EventResult::Ignored(None),
            Event::Paste(_) | Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => {
                return EventResult::Ignored(None)
            }
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _| {
            // remove the layer
            compositor.last_picker = compositor.pop();
        })));

        let findfile_fn = |path: &Path| {
            let picker = FindFilePicker::new(path.to_path_buf());
            EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _| {
                // remove the layer
                compositor.last_picker = compositor.pop();
                compositor.push(Box::new(overlay::overlayed(picker)));
            })))
        };

        // different from FilePicker callback_fn as in second option is an
        // Option<T> and returns EventResult
        let callback_fn =
            move |cx: &mut Context, picker: &FilePicker<PathBuf>, dir: &PathBuf, action| {
                if let Some(path) = picker.picker.selection() {
                    if path.is_dir() {
                        return findfile_fn(path);
                    } else {
                        cx.editor.open(path, action).expect("editor.open failed");
                    }
                } else {
                    let filename = picker.picker.prompt.line();
                    cx.editor
                        .open(&dir.join(filename), action)
                        .expect("editor.open failed");
                }
                close_fn
            };

        match key {
            ctrl!('h') | key!(Backspace) if self.picker.picker.prompt.line().is_empty() => {
                let parent = self.dir.parent().unwrap_or(&self.dir);
                findfile_fn(parent)
            }
            key!(Enter) => (callback_fn)(cx, &self.picker, &self.dir, Action::Replace),
            ctrl!('s') => (callback_fn)(cx, &self.picker, &self.dir, Action::HorizontalSplit),
            ctrl!('v') => (callback_fn)(cx, &self.picker, &self.dir, Action::VerticalSplit),
            _ => self.picker.handle_event(event, cx),
        }
    }
}

pub struct FilePicker<T: Item> {
    picker: Picker<T>,
    pub truncate_start: bool,
    /// Caches paths to documents
    preview_cache: HashMap<PathBuf, CachedPreview>,
    read_buffer: Vec<u8>,
    /// Given an item in the picker, return the file path and line number to display.
    file_fn: Box<dyn Fn(&Editor, &T) -> Option<FileLocation>>,
}

impl<T: Item> FilePicker<T> {
    pub fn new(
        options: Vec<T>,
        editor_data: T::Data,
        callback_fn: impl Fn(&mut Context, &T, Action) + 'static,
        preview_fn: impl Fn(&Editor, &T) -> Option<FileLocation> + 'static,
    ) -> Self {
        let truncate_start = true;
        let mut picker = Picker::new(options, editor_data, callback_fn);
        picker.truncate_start = truncate_start;

        Self {
            picker,
            truncate_start,
            preview_cache: HashMap::new(),
            read_buffer: Vec::with_capacity(1024),
            file_fn: Box::new(preview_fn),
        }
    }

    pub fn truncate_start(mut self, truncate_start: bool) -> Self {
        self.truncate_start = truncate_start;
        self.picker.truncate_start = truncate_start;
        self
    }

    fn current_file(&self, editor: &Editor) -> Option<FileLocation> {
        self.picker
            .selection()
            .and_then(|current| (self.file_fn)(editor, current))
            .and_then(|(path, line)| {
                helix_core::path::get_canonicalized_path(&path)
                    .ok()
                    .zip(Some(line))
            })
    }

    /// Get (cached) preview for a given path. If a document corresponding
    /// to the path is already open in the editor, it is used instead.
    fn get_preview<'picker, 'editor>(
        &'picker mut self,
        path: &Path,
        editor: &'editor Editor,
    ) -> Preview<'picker, 'editor> {
        if let Some(doc) = editor.document_by_path(path) {
            return Preview::EditorDocument(doc);
        }

        if self.preview_cache.contains_key(path) {
            return Preview::Cached(&self.preview_cache[path]);
        }

        let preview = std::fs::File::open(path)
            .and_then(|file| {
                let metadata = file.metadata()?;
                if metadata.is_file() {
                    // Read up to 1kb to detect the content type
                    let n = file.take(1024).read_to_end(&mut self.read_buffer)?;
                    let content_type = content_inspector::inspect(&self.read_buffer[..n]);
                    self.read_buffer.clear();
                    Ok(match (metadata.len(), content_type) {
                        (_, content_inspector::ContentType::BINARY) => CachedPreview::Binary,
                        (size, _) if size > MAX_FILE_SIZE_FOR_PREVIEW => CachedPreview::LargeFile,
                        _ => {
                            // TODO: enable syntax highlighting; blocked by async rendering
                            Document::open(path, None, None)
                                .map(|doc| CachedPreview::Document(Box::new(doc)))
                                .unwrap_or(CachedPreview::NotFound)
                        }
                    })
                } else {
                    // metadata.is_dir
                    Ok(CachedPreview::Directory(self.preview_dir(path)))
                }
            })
            .unwrap_or(CachedPreview::NotFound);
        self.preview_cache.insert(path.to_owned(), preview);
        Preview::Cached(&self.preview_cache[path])
    }

    fn preview_dir(&self, dir: &Path) -> Vec<String> {
        // TODO refactor this with directory list to directory.rs in view later
        match fs::read_dir(&dir) {
            Ok(entries) => entries
                .flat_map(|res| {
                    res.map(|entry| {
                        let path = entry.path();
                        let suffix = if path.is_dir() { "/" } else { "" };
                        let metadata = fs::metadata(&*path);
                        let path = path.strip_prefix(&dir).unwrap_or(&path).to_string_lossy();
                        if cfg!(unix) {
                            if let Ok(metadata) = metadata {
                                let filetype = fields::filetype(&metadata);
                                let permissions = fields::permissions(&metadata);
                                let size = format!("{}", fields::size(&metadata));
                                format!(
                                    "{:<22} {}{} {:>6}",
                                    path + suffix, // TODO this should check for size and handle truncation
                                    filetype,
                                    permissions,
                                    size,
                                    // TODO add absolute/relative time? may need to handle truncation
                                )
                            } else {
                                // metadata fails in cases like broken soft link
                                (path + suffix).to_string()
                            }
                        } else {
                            (path + suffix).to_string()
                        }
                    })
                })
                .collect(),
            Err(_) => vec!["<No access to directory>".to_owned()],
        }
    }
}

impl<T: Item + 'static> Component for FilePicker<T> {
    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // +---------+ +---------+
        // |prompt   | |preview  |
        // +---------+ |         |
        // |picker   | |         |
        // |         | |         |
        // +---------+ +---------+

        let render_preview = self.picker.show_preview && area.width > MIN_AREA_WIDTH_FOR_PREVIEW;
        // -- Render the frame:
        // clear area
        let background = cx.editor.theme.get("ui.background");
        let text = cx.editor.theme.get("ui.text");
        surface.clear_with(area, background);

        let picker_width = if render_preview {
            area.width / 2
        } else {
            area.width
        };

        let picker_area = area.with_width(picker_width);
        self.picker.render(picker_area, surface, cx);

        if !render_preview {
            return;
        }

        let preview_area = area.clip_left(picker_width);

        // don't like this but the lifetime sucks
        let block = Block::default().borders(Borders::ALL);

        // calculate the inner area inside the box
        let inner = block.inner(preview_area);
        // 1 column gap on either side
        let margin = Margin::horizontal(1);
        let inner = inner.inner(&margin);
        block.render(preview_area, surface);

        if let Some((path, range)) = self.current_file(cx.editor) {
            let preview = self.get_preview(&path, cx.editor);
            let doc = if let Some(doc) = preview.document() {
                doc
            } else if let Some(output) = preview.directory() {
                for (n, line) in output.iter().take(inner.height as usize).enumerate() {
                    let y = inner.y + n as u16;
                    surface.set_stringn(inner.x, y, line, inner.width as usize, text);
                }
                return;
            } else {
                let alt_text = preview.placeholder();
                let x = inner.x + inner.width.saturating_sub(alt_text.len() as u16) / 2;
                let y = inner.y + inner.height / 2;
                surface.set_stringn(x, y, alt_text, inner.width as usize, text);
                return;
            };

            // align to middle
            let first_line = range
                .map(|(start, end)| {
                    let height = end.saturating_sub(start) + 1;
                    let middle = start + (height.saturating_sub(1) / 2);
                    middle.saturating_sub(inner.height as usize / 2).min(start)
                })
                .unwrap_or(0);

            let offset = Position::new(first_line, 0);

            let highlights =
                EditorView::doc_syntax_highlights(doc, offset, area.height, &cx.editor.theme);
            EditorView::render_text_highlights(
                doc,
                offset,
                inner,
                surface,
                &cx.editor.theme,
                highlights,
                &cx.editor.config(),
            );

            // highlight the line
            if let Some((start, end)) = range {
                let offset = start.saturating_sub(first_line) as u16;
                surface.set_style(
                    Rect::new(
                        inner.x,
                        inner.y + offset,
                        inner.width,
                        (end.saturating_sub(start) as u16 + 1)
                            .min(inner.height.saturating_sub(offset)),
                    ),
                    cx.editor
                        .theme
                        .try_get("ui.highlight")
                        .unwrap_or_else(|| cx.editor.theme.get("ui.selection")),
                );
            }
        }
    }

    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult {
        // TODO: keybinds for scrolling preview
        self.picker.handle_event(event, ctx)
    }

    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind) {
        self.picker.cursor(area, ctx)
    }

    fn required_size(&mut self, (width, height): (u16, u16)) -> Option<(u16, u16)> {
        let picker_width = if width > MIN_AREA_WIDTH_FOR_PREVIEW {
            width / 2
        } else {
            width
        };
        self.picker.required_size((picker_width, height))?;
        Some((width, height))
    }
}

pub struct Picker<T: Item> {
    options: Vec<T>,
    editor_data: T::Data,
    // filter: String,
    matcher: Box<Matcher>,
    /// (index, score)
    matches: Vec<(usize, i64)>,
    /// Filter over original options.
    filters: Vec<usize>, // could be optimized into bit but not worth it now

    /// Current height of the completions box
    completion_height: u16,

    cursor: usize,
    // pattern: String,
    prompt: Prompt,
    previous_pattern: String,
    /// Whether to truncate the start (default true)
    pub truncate_start: bool,
    /// Whether to show the preview panel (default true)
    show_preview: bool,

    callback_fn: Box<dyn Fn(&mut Context, &T, Action)>,
}

impl<T: Item> Picker<T> {
    pub fn new(
        options: Vec<T>,
        editor_data: T::Data,
        callback_fn: impl Fn(&mut Context, &T, Action) + 'static,
    ) -> Self {
        let prompt = Prompt::new(
            "".into(),
            None,
            ui::completers::none,
            |_editor: &mut Context, _pattern: &str, _event: PromptEvent| {},
        );

        let mut picker = Self {
            options,
            editor_data,
            matcher: Box::new(Matcher::default()),
            matches: Vec::new(),
            filters: Vec::new(),
            cursor: 0,
            prompt,
            previous_pattern: String::new(),
            truncate_start: true,
            show_preview: true,
            callback_fn: Box::new(callback_fn),
            completion_height: 0,
        };

        // scoring on empty input:
        // TODO: just reuse score()
        picker.matches.extend(
            picker
                .options
                .iter()
                .enumerate()
                .map(|(index, _option)| (index, 0)),
        );

        picker
    }

    pub fn score(&mut self) {
        let now = Instant::now();

        let pattern = self.prompt.line();

        if pattern == &self.previous_pattern {
            return;
        }

        if pattern.is_empty() {
            // Fast path for no pattern.
            self.matches.clear();
            self.matches.extend(
                self.options
                    .iter()
                    .enumerate()
                    .map(|(index, _option)| (index, 0)),
            );
        } else if pattern.starts_with(&self.previous_pattern) {
            // optimization: if the pattern is a more specific version of the previous one
            // then we can score the filtered set.
            self.matches.retain_mut(|(index, score)| {
                let option = &self.options[*index];
                let text = option.sort_text(&self.editor_data);

                match self.matcher.fuzzy_match(&text, pattern) {
                    Some(s) => {
                        // Update the score
                        *score = s;
                        true
                    }
                    None => false,
                }
            });

            self.matches
                .sort_unstable_by_key(|(_, score)| Reverse(*score));
        } else {
            self.matches.clear();
            self.matches.extend(
                self.options
                    .iter()
                    .enumerate()
                    .filter_map(|(index, option)| {
                        // filter options first before matching
                        if !self.filters.is_empty() {
                            // TODO: this filters functionality seems inefficient,
                            // instead store and operate on filters if any
                            self.filters.binary_search(&index).ok()?;
                        }

                        let text = option.filter_text(&self.editor_data);

                        self.matcher
                            .fuzzy_match(&text, pattern)
                            .map(|score| (index, score))
                    }),
            );
            self.matches
                .sort_unstable_by_key(|(_, score)| Reverse(*score));
        }

        log::debug!("picker score {:?}", Instant::now().duration_since(now));

        // reset cursor position
        self.cursor = 0;
        self.previous_pattern.clone_from(pattern);
    }

    /// Move the cursor by a number of lines, either down (`Forward`) or up (`Backward`)
    pub fn move_by(&mut self, amount: usize, direction: Direction) {
        let len = self.matches.len();

        if len == 0 {
            // No results, can't move.
            return;
        }

        match direction {
            Direction::Forward => {
                self.cursor = self.cursor.saturating_add(amount) % len;
            }
            Direction::Backward => {
                self.cursor = self.cursor.saturating_add(len).saturating_sub(amount) % len;
            }
        }
    }

    /// Move the cursor down by exactly one page. After the last page comes the first page.
    pub fn page_up(&mut self) {
        self.move_by(self.completion_height as usize, Direction::Backward);
    }

    /// Move the cursor up by exactly one page. After the first page comes the last page.
    pub fn page_down(&mut self) {
        self.move_by(self.completion_height as usize, Direction::Forward);
    }

    /// Move the cursor to the first entry
    pub fn to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move the cursor to the last entry
    pub fn to_end(&mut self) {
        self.cursor = self.matches.len().saturating_sub(1);
    }

    pub fn selection(&self) -> Option<&T> {
        self.matches
            .get(self.cursor)
            .map(|(index, _score)| &self.options[*index])
    }

    pub fn save_filter(&mut self, cx: &Context) {
        self.filters.clear();
        self.filters
            .extend(self.matches.iter().map(|(index, _)| *index));
        self.filters.sort_unstable(); // used for binary search later
        self.prompt.clear(cx.editor);
    }

    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
    }

    fn prompt_handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        if let EventResult::Consumed(_) = self.prompt.handle_event(event, cx) {
            // TODO: recalculate only if pattern changed
            self.score();
        }
        EventResult::Consumed(None)
    }
}

// process:
// - read all the files into a list, maxed out at a large value
// - on input change:
//  - score all the names in relation to input

impl<T: Item + 'static> Component for Picker<T> {
    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.completion_height = viewport.1.saturating_sub(4);
        Some(viewport)
    }

    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => *event,
            Event::Paste(..) => return self.prompt_handle_event(event, cx),
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored(None),
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _cx| {
            // remove the layer
            compositor.last_picker = compositor.pop();
        })));

        match key_event {
            shift!(Tab) | key!(Up) | ctrl!('p') => {
                self.move_by(1, Direction::Backward);
            }
            key!(Tab) | key!(Down) | ctrl!('n') => {
                self.move_by(1, Direction::Forward);
            }
            key!(PageDown) | ctrl!('d') => {
                self.page_down();
            }
            key!(PageUp) | ctrl!('u') => {
                self.page_up();
            }
            key!(Home) => {
                self.to_start();
            }
            key!(End) => {
                self.to_end();
            }
            key!(Esc) | ctrl!('c') => {
                return close_fn;
            }
            key!(Enter) => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(cx, option, Action::Replace);
                }
                return close_fn;
            }
            ctrl!('s') => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(cx, option, Action::HorizontalSplit);
                }
                return close_fn;
            }
            ctrl!('v') => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(cx, option, Action::VerticalSplit);
                }
                return close_fn;
            }
            ctrl!(' ') => {
                self.save_filter(cx);
            }
            ctrl!('t') => {
                self.toggle_preview();
            }
            _ => {
                self.prompt_handle_event(event, cx);
            }
        }

        EventResult::Consumed(None)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let text_style = cx.editor.theme.get("ui.text");
        let selected = cx.editor.theme.get("ui.text.focus");
        let highlighted = cx.editor.theme.get("special").add_modifier(Modifier::BOLD);

        // -- Render the frame:
        // clear area
        let background = cx.editor.theme.get("ui.background");
        surface.clear_with(area, background);

        // don't like this but the lifetime sucks
        let block = Block::default().borders(Borders::ALL);

        // calculate the inner area inside the box
        let inner = block.inner(area);

        block.render(area, surface);

        // -- Render the input bar:

        let area = inner.clip_left(1).with_height(1);

        let count = format!("{}/{}", self.matches.len(), self.options.len());
        surface.set_stringn(
            (area.x + area.width).saturating_sub(count.len() as u16 + 1),
            area.y,
            &count,
            (count.len()).min(area.width as usize),
            text_style,
        );

        self.prompt.render(area, surface, cx);

        // -- Separator
        let sep_style = cx.editor.theme.get("ui.background.separator");
        let borders = BorderType::line_symbols(BorderType::Plain);
        for x in inner.left()..inner.right() {
            if let Some(cell) = surface.get_mut(x, inner.y + 1) {
                cell.set_symbol(borders.horizontal).set_style(sep_style);
            }
        }

        // -- Render the contents:
        // subtract area of prompt from top and current item marker " > " from left
        let inner = inner.clip_top(2).clip_left(3);

        let rows = inner.height;
        let offset = self.cursor - (self.cursor % std::cmp::max(1, rows as usize));

        let files = self
            .matches
            .iter()
            .skip(offset)
            .map(|(index, _score)| (*index, self.options.get(*index).unwrap()));

        for (i, (_index, option)) in files.take(rows as usize).enumerate() {
            let is_active = i == (self.cursor - offset);
            if is_active {
                surface.set_string(
                    inner.x.saturating_sub(3),
                    inner.y + i as u16,
                    " > ",
                    selected,
                );
                surface.set_style(
                    Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
                    selected,
                );
            }

            let spans = option.label(&self.editor_data);
            let (_score, highlights) = self
                .matcher
                .fuzzy_indices(&String::from(&spans), self.prompt.line())
                .unwrap_or_default();

            spans.0.into_iter().fold(inner, |pos, span| {
                let new_x = surface
                    .set_string_truncated(
                        pos.x,
                        pos.y + i as u16,
                        &span.content,
                        pos.width as usize,
                        |idx| {
                            if highlights.contains(&idx) {
                                highlighted.patch(span.style)
                            } else if is_active {
                                selected.patch(span.style)
                            } else {
                                text_style.patch(span.style)
                            }
                        },
                        true,
                        self.truncate_start,
                    )
                    .0;
                pos.clip_left(new_x - pos.x)
            });
        }
    }

    fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        let block = Block::default().borders(Borders::ALL);
        // calculate the inner area inside the box
        let inner = block.inner(area);

        // prompt area
        let area = inner.clip_left(1).with_height(1);

        self.prompt.cursor(area, editor)
    }
}
