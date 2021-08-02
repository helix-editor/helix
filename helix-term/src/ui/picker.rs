use crate::{
    commands::{self, Align},
    compositor::{Component, Compositor, Context, EventResult},
    ui::EditorView,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Borders},
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;
use tui::widgets::Widget;

use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use crate::ui::{Prompt, PromptEvent};
use helix_core::{hashmap, Position, Range, Selection};
use helix_view::{
    document::canonicalize_path,
    editor::Action,
    graphics::{Color, CursorKind, Rect, Style},
    Document, Editor, View,
};

pub struct PreviewedPicker<T> {
    picker: Picker<T>,
    // Caches paths to docs to line number to view
    preview_cache: HashMap<PathBuf, (Document, HashMap<usize, View>)>,
    #[allow(clippy::type_complexity)]
    preview_fn: Box<dyn Fn(&Editor, &T) -> Option<(PathBuf, usize)>>,
}

impl<T> PreviewedPicker<T> {
    pub fn new(
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, &T, Action) + 'static,
        preview_fn: impl Fn(&Editor, &T) -> Option<(PathBuf, usize)> + 'static,
    ) -> Self {
        Self {
            picker: Picker::new(options, format_fn, callback_fn),
            preview_cache: HashMap::new(),
            preview_fn: Box::new(preview_fn),
        }
    }

    fn calculate_preview(&mut self, editor: &Editor) {
        if let Some((path, line)) = self
            .picker
            .selection()
            .and_then(|current| (self.preview_fn)(editor, current))
            .and_then(|(path, line)| canonicalize_path(&path).ok().zip(Some(line)))
        {
            let &mut (ref mut doc, ref mut range_map) =
                self.preview_cache.entry(path.clone()).or_insert_with(|| {
                    let doc =
                        Document::open(path, None, Some(&editor.theme), Some(&editor.syn_loader))
                            .unwrap();
                    let view = View::new(doc.id());
                    (doc, hashmap!(line => view))
                });
            let view = range_map.entry(line).or_insert_with(|| View::new(doc.id()));

            let range = Range::point(doc.text().line_to_char(line));
            doc.set_selection(view.id, Selection::from(range));
            // FIXME: gets aligned top instead of center
            commands::align_view(doc, view, Align::Center);
        }
    }
}

impl<T: 'static> Component for PreviewedPicker<T> {
    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let area = inner_rect(area);
        // -- Render the frame:
        // clear area
        let background = cx.editor.theme.get("ui.background");
        surface.clear_with(area, background);

        let picker_area = Rect::new(area.x, area.y, area.width / 2, area.height);
        let preview_area = Rect::new(
            area.x + picker_area.width,
            area.y,
            area.width / 2,
            area.height,
        );
        self.picker.render(picker_area, surface, cx);

        // don't like this but the lifetime sucks
        let block = Block::default().borders(Borders::ALL);

        // calculate the inner area inside the box
        let inner = block.inner(preview_area);

        block.render(preview_area, surface);

        if let Some((doc, view)) = self
            .picker
            .selection()
            .and_then(|current| (self.preview_fn)(cx.editor, current))
            .and_then(|(path, line)| canonicalize_path(&path).ok().zip(Some(line)))
            .and_then(|(path, line)| {
                self.preview_cache
                    .get(&path)
                    .and_then(|(doc, range_map)| Some((doc, range_map.get(&line)?)))
            })
        {
            // FIXME: last line will not be highlighted because of a -1 in View::last_line
            let mut view = view.clone();
            view.area = inner;
            EditorView::render_doc(
                doc,
                &view,
                inner,
                surface,
                &cx.editor.theme,
                true, // is_focused
                &cx.editor.syn_loader,
            );
        }
    }

    fn prepare_for_render(&mut self, cx: &Context) {
        self.calculate_preview(cx.editor);
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

    format_fn: Box<dyn Fn(&T) -> Cow<str>>,
    callback_fn: Box<dyn Fn(&mut Editor, &T, Action)>,
}

impl<T> Picker<T> {
    pub fn new(
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, &T, Action) + 'static,
    ) -> Self {
        let prompt = Prompt::new(
            "".to_string(),
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
            format_fn: Box::new(format_fn),
            callback_fn: Box::new(callback_fn),
        };

        // TODO: scoring on empty input should just use a fastpath
        picker.score();

        picker
    }

    pub fn score(&mut self) {
        // need to borrow via pattern match otherwise it complains about simultaneous borrow
        let Self {
            ref mut matcher,
            ref mut matches,
            ref filters,
            ref format_fn,
            ..
        } = *self;

        let pattern = &self.prompt.line;

        // reuse the matches allocation
        matches.clear();
        matches.extend(
            self.options
                .iter()
                .enumerate()
                .filter_map(|(index, option)| {
                    // filter options first before matching
                    if !filters.is_empty() {
                        filters.binary_search(&index).ok()?;
                    }
                    // TODO: maybe using format_fn isn't the best idea here
                    let text = (format_fn)(option);
                    // TODO: using fuzzy_indices could give us the char idx for match highlighting
                    matcher
                        .fuzzy_match(&text, pattern)
                        .map(|score| (index, score))
                }),
        );
        matches.sort_unstable_by_key(|(_, score)| -score);

        // reset cursor position
        self.cursor = 0;
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        if self.cursor < self.matches.len() - 1 {
            self.cursor += 1;
        }
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
    let padding_vertical = area.height * 20 / 100;
    let padding_horizontal = area.width * 20 / 100;

    Rect::new(
        area.x + padding_horizontal,
        area.y + padding_vertical,
        area.width - padding_horizontal * 2,
        area.height - padding_vertical * 2,
    )
}

impl<T: 'static> Component for Picker<T> {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor| {
            // remove the layer
            compositor.last_picker = compositor.pop();
        })));

        match key_event {
            KeyEvent {
                code: KeyCode::Up, ..
            }
            | KeyEvent {
                code: KeyCode::BackTab,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.move_up();
            }
            KeyEvent {
                code: KeyCode::Down,
                ..
            }
            | KeyEvent {
                code: KeyCode::Tab, ..
            }
            | KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.move_down();
            }
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::Replace);
                }
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::HorizontalSplit);
                }
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::VerticalSplit);
                }
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::CONTROL,
            } => {
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

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
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

        let area = Rect::new(inner.x + 1, inner.y, inner.width - 1, 1);
        self.prompt.render(area, surface, cx);

        // -- Separator
        let sep_style = Style::default().fg(Color::Rgb(90, 89, 119));
        let borders = BorderType::line_symbols(BorderType::Plain);
        for x in inner.left()..inner.right() {
            surface
                .get_mut(x, inner.y + 1)
                .set_symbol(borders.horizontal)
                .set_style(sep_style);
        }

        // -- Render the contents:
        // subtract the area of the prompt (-2) and current item marker " > " (-3)
        let inner = Rect::new(inner.x + 3, inner.y + 2, inner.width - 3, inner.height - 2);

        let style = cx.editor.theme.get("ui.text");
        let selected = Style::default().fg(Color::Rgb(255, 255, 255));

        let rows = inner.height;
        let offset = self.cursor / (rows as usize) * (rows as usize);

        let files = self.matches.iter().skip(offset).map(|(index, _score)| {
            (index, self.options.get(*index).unwrap()) // get_unchecked
        });

        for (i, (_index, option)) in files.take(rows as usize).enumerate() {
            if i == (self.cursor - offset) {
                surface.set_string(inner.x - 2, inner.y + i as u16, ">", selected);
            }

            surface.set_string_truncated(
                inner.x,
                inner.y + i as u16,
                (self.format_fn)(option),
                inner.width as usize,
                if i == (self.cursor - offset) {
                    selected
                } else {
                    style
                },
                true,
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
        let area = Rect::new(inner.x + 1, inner.y, inner.width - 1, 1);

        self.prompt.cursor(area, editor)
    }
}
