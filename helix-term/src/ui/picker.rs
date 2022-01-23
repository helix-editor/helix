use crate::{
    compositor::{Component, Compositor, Context, EventResult},
    ctrl, key, shift,
    ui::EditorView,
};
use crossterm::event::Event;
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Borders},
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;
use tui::widgets::Widget;

use std::{
    borrow::Cow,
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

use crate::ui::{Prompt, PromptEvent};
use helix_core::Position;
use helix_view::{
    editor::Action,
    graphics::{Color, CursorKind, Margin, Rect, Style},
    Document, Editor,
};

pub const MIN_SCREEN_WIDTH_FOR_PREVIEW: u16 = 80;
/// Biggest file size to preview in bytes
pub const MAX_FILE_SIZE_FOR_PREVIEW: u64 = 10 * 1024 * 1024;

/// File path and range of lines (used to align and highlight lines)
type FileLocation = (PathBuf, Option<(usize, usize)>);

pub struct FilePicker<T> {
    picker: Picker<T>,
    pub truncate_start: bool,
    /// Caches paths to documents
    preview_cache: HashMap<PathBuf, CachedPreview>,
    read_buffer: Vec<u8>,
    /// Given an item in the picker, return the file path and line number to display.
    file_fn: Box<dyn Fn(&Editor, &T) -> Option<FileLocation>>,
}

pub enum CachedPreview {
    Document(Box<Document>),
    Binary,
    LargeFile,
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

    /// Alternate text to show for the preview.
    fn placeholder(&self) -> &str {
        match *self {
            Self::EditorDocument(_) => "<File preview>",
            Self::Cached(preview) => match preview {
                CachedPreview::Document(_) => "<File preview>",
                CachedPreview::Binary => "<Binary file>",
                CachedPreview::LargeFile => "<File too large to preview>",
                CachedPreview::NotFound => "<File not found>",
            },
        }
    }
}

impl<T> FilePicker<T> {
    pub fn new(
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, &T, Action) + 'static,
        preview_fn: impl Fn(&Editor, &T) -> Option<FileLocation> + 'static,
    ) -> Self {
        Self {
            picker: Picker::new(false, options, format_fn, callback_fn),
            truncate_start: true,
            preview_cache: HashMap::new(),
            read_buffer: Vec::with_capacity(1024),
            file_fn: Box::new(preview_fn),
        }
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

        let data = std::fs::File::open(path).and_then(|file| {
            let metadata = file.metadata()?;
            // Read up to 1kb to detect the content type
            let n = file.take(1024).read_to_end(&mut self.read_buffer)?;
            let content_type = content_inspector::inspect(&self.read_buffer[..n]);
            self.read_buffer.clear();
            Ok((metadata, content_type))
        });
        let preview = data
            .map(
                |(metadata, content_type)| match (metadata.len(), content_type) {
                    (_, content_inspector::ContentType::BINARY) => CachedPreview::Binary,
                    (size, _) if size > MAX_FILE_SIZE_FOR_PREVIEW => CachedPreview::LargeFile,
                    _ => {
                        // TODO: enable syntax highlighting; blocked by async rendering
                        Document::open(path, None, None)
                            .map(|doc| CachedPreview::Document(Box::new(doc)))
                            .unwrap_or(CachedPreview::NotFound)
                    }
                },
            )
            .unwrap_or(CachedPreview::NotFound);
        self.preview_cache.insert(path.to_owned(), preview);
        Preview::Cached(&self.preview_cache[path])
    }
}

impl<T: 'static> Component for FilePicker<T> {
    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // +---------+ +---------+
        // |prompt   | |preview  |
        // +---------+ |         |
        // |picker   | |         |
        // |         | |         |
        // +---------+ +---------+

        let render_preview = area.width > MIN_SCREEN_WIDTH_FOR_PREVIEW;
        let area = inner_rect(area);
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
        self.picker.truncate_start = self.truncate_start;
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
        let margin = Margin {
            vertical: 0,
            horizontal: 1,
        };
        let inner = inner.inner(&margin);
        block.render(preview_area, surface);

        if let Some((path, range)) = self.current_file(cx.editor) {
            let preview = self.get_preview(&path, cx.editor);
            let doc = match preview.document() {
                Some(doc) => doc,
                None => {
                    let alt_text = preview.placeholder();
                    let x = inner.x + inner.width.saturating_sub(alt_text.len() as u16) / 2;
                    let y = inner.y + inner.height / 2;
                    surface.set_stringn(x, y, alt_text, inner.width as usize, text);
                    return;
                }
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

    fn handle_event(&mut self, event: Event, ctx: &mut Context) -> EventResult {
        // TODO: keybinds for scrolling preview
        self.picker.handle_event(event, ctx)
    }

    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind) {
        self.picker.cursor(area, ctx)
    }
}

pub struct Picker<T> {
    options: Vec<T>,
    // filter: String,
    matcher: Box<Matcher>,
    /// (index, score)
    matches: Vec<(usize, i64)>,
    /// Filter over original options.
    filters: Vec<usize>, // could be optimized into bit but not worth it now

    cursor: usize,
    // pattern: String,
    prompt: Prompt,
    /// Whether to render in the middle of the area
    render_centered: bool,
    /// Wheather to truncate the start (default true)
    pub truncate_start: bool,

    format_fn: Box<dyn Fn(&T) -> Cow<str>>,
    callback_fn: Box<dyn Fn(&mut Editor, &T, Action)>,
}

impl<T> Picker<T> {
    pub fn new(
        render_centered: bool,
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, &T, Action) + 'static,
    ) -> Self {
        let prompt = Prompt::new(
            "".into(),
            None,
            |_pattern: &str| Vec::new(),
            |_editor: &mut Context, _pattern: &str, _event: PromptEvent| {
                //
            },
        );

        let mut picker = Self {
            options,
            matcher: Box::new(Matcher::default()),
            matches: Vec::new(),
            filters: Vec::new(),
            cursor: 0,
            prompt,
            render_centered,
            truncate_start: true,
            format_fn: Box::new(format_fn),
            callback_fn: Box::new(callback_fn),
        };

        // TODO: scoring on empty input should just use a fastpath
        picker.score();

        picker
    }

    pub fn score(&mut self) {
        let pattern = &self.prompt.line;

        // reuse the matches allocation
        self.matches.clear();
        self.matches.extend(
            self.options
                .iter()
                .enumerate()
                .filter_map(|(index, option)| {
                    // filter options first before matching
                    if !self.filters.is_empty() {
                        self.filters.binary_search(&index).ok()?;
                    }
                    // TODO: maybe using format_fn isn't the best idea here
                    let text = (self.format_fn)(option);
                    // TODO: using fuzzy_indices could give us the char idx for match highlighting
                    self.matcher
                        .fuzzy_match(&text, pattern)
                        .map(|score| (index, score))
                }),
        );
        self.matches.sort_unstable_by_key(|(_, score)| -score);

        // reset cursor position
        self.cursor = 0;
    }

    pub fn move_up(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len();
        let pos = ((self.cursor + len.saturating_sub(1)) % len) % len;
        self.cursor = pos;
    }

    pub fn move_down(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len();
        let pos = (self.cursor + 1) % len;
        self.cursor = pos;
    }

    pub fn selection(&self) -> Option<&T> {
        self.matches
            .get(self.cursor)
            .map(|(index, _score)| &self.options[*index])
    }

    pub fn save_filter(&mut self) {
        self.filters.clear();
        self.filters
            .extend(self.matches.iter().map(|(index, _)| *index));
        self.filters.sort_unstable(); // used for binary search later
        self.prompt.clear();
    }
}

// process:
// - read all the files into a list, maxed out at a large value
// - on input change:
//  - score all the names in relation to input

fn inner_rect(area: Rect) -> Rect {
    let margin = Margin {
        vertical: area.height * 10 / 100,
        horizontal: area.width * 10 / 100,
    };
    area.inner(&margin)
}

impl<T: 'static> Component for Picker<T> {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _| {
            // remove the layer
            compositor.last_picker = compositor.pop();
        })));

        match key_event.into() {
            shift!(Tab) | key!(Up) | ctrl!('p') | ctrl!('k') => {
                self.move_up();
            }
            key!(Tab) | key!(Down) | ctrl!('n') | ctrl!('j') => {
                self.move_down();
            }
            key!(Esc) | ctrl!('c') => {
                return close_fn;
            }
            key!(Enter) => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(cx.editor, option, Action::Replace);
                }
                return close_fn;
            }
            ctrl!('s') => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(cx.editor, option, Action::HorizontalSplit);
                }
                return close_fn;
            }
            ctrl!('v') => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(cx.editor, option, Action::VerticalSplit);
                }
                return close_fn;
            }
            ctrl!(' ') => {
                self.save_filter();
            }
            _ => {
                if let EventResult::Consumed(_) = self.prompt.handle_event(event, cx) {
                    // TODO: recalculate only if pattern changed
                    self.score();
                }
            }
        }

        EventResult::Consumed(None)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let area = if self.render_centered {
            inner_rect(area)
        } else {
            area
        };

        let text_style = cx.editor.theme.get("ui.text");

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
        let sep_style = Style::default().fg(Color::Rgb(90, 89, 119));
        let borders = BorderType::line_symbols(BorderType::Plain);
        for x in inner.left()..inner.right() {
            if let Some(cell) = surface.get_mut(x, inner.y + 1) {
                cell.set_symbol(borders.horizontal).set_style(sep_style);
            }
        }

        // -- Render the contents:
        // subtract area of prompt from top and current item marker " > " from left
        let inner = inner.clip_top(2).clip_left(3);

        let selected = cx.editor.theme.get("ui.text.focus");

        let rows = inner.height;
        let offset = self.cursor - (self.cursor % std::cmp::max(1, rows as usize));

        let files = self.matches.iter().skip(offset).map(|(index, _score)| {
            (index, self.options.get(*index).unwrap()) // get_unchecked
        });

        for (i, (_index, option)) in files.take(rows as usize).enumerate() {
            if i == (self.cursor - offset) {
                surface.set_string(inner.x.saturating_sub(2), inner.y + i as u16, ">", selected);
            }

            surface.set_string_truncated(
                inner.x,
                inner.y + i as u16,
                (self.format_fn)(option),
                inner.width as usize,
                if i == (self.cursor - offset) {
                    selected
                } else {
                    text_style
                },
                true,
                self.truncate_start,
            );
        }
    }

    fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        // TODO: this is mostly duplicate code
        let area = inner_rect(area);
        let block = Block::default().borders(Borders::ALL);
        // calculate the inner area inside the box
        let inner = block.inner(area);

        // prompt area
        let area = inner.clip_left(1).with_height(1);

        self.prompt.cursor(area, editor)
    }
}
