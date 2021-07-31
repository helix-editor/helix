use crate::{
    commands::{self, Align},
    compositor::{Component, Compositor, Context, EventResult},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, BorderType, Borders},
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;

use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use crate::ui::{Prompt, PromptEvent};
use helix_core::{Position, Range, Selection};
use helix_view::{
    document::canonicalize_path,
    editor::Action,
    graphics::{Color, CursorKind, Rect, Style},
    Document, Editor, View,
};

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
    preview_cache: HashMap<(PathBuf, Range), (Document, View)>,

    format_fn: Box<dyn Fn(&T) -> Cow<str>>,
    callback_fn: Box<dyn Fn(&mut Editor, &T, Action)>,
    preview_fn: Box<dyn Fn(&T) -> Option<(PathBuf, Range)>>,
}

impl<T> Picker<T> {
    pub fn new(
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, &T, Action) + 'static,
        preview_fn: impl Fn(&T) -> Option<(PathBuf, Range)> + 'static,
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
            preview_cache: HashMap::new(),
            format_fn: Box::new(format_fn),
            callback_fn: Box::new(callback_fn),
            preview_fn: Box::new(preview_fn),
        };

        // TODO: scoring on empty input should just use a fastpath
        picker.score();

        picker
    }

    // TODO: Copied from EditorView::render_buffer, reuse
    #[allow(clippy::too_many_arguments)]
    fn render_buffer(
        &self,
        doc: &Document,
        view: &helix_view::View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &helix_view::Theme,
        loader: &helix_core::syntax::Loader,
    ) {
        let text = doc.text().slice(..);

        let last_line = view.last_line(doc);

        let range = {
            // calculate viewport byte ranges
            let start = text.line_to_byte(view.first_line);
            let end = text.line_to_byte(last_line + 1);

            start..end
        };

        // TODO: range doesn't actually restrict source, just highlight range
        let highlights: Vec<_> = match doc.syntax() {
            Some(syntax) => {
                let scopes = theme.scopes();
                syntax
                    .highlight_iter(text.slice(..), Some(range), None, |language| {
                        loader
                            .language_config_for_scope(&format!("source.{}", language))
                            .and_then(|language_config| {
                                let config = language_config.highlight_config(scopes)?;
                                let config_ref = config.as_ref();
                                // SAFETY: the referenced `HighlightConfiguration` behind
                                // the `Arc` is guaranteed to remain valid throughout the
                                // duration of the highlight.
                                let config_ref = unsafe {
                                    std::mem::transmute::<
                                        _,
                                        &'static helix_core::syntax::HighlightConfiguration,
                                    >(config_ref)
                                };
                                Some(config_ref)
                            })
                    })
                    .collect() // TODO: we collect here to avoid holding the lock, fix later
            }
            None => vec![Ok(helix_core::syntax::HighlightEvent::Source {
                start: range.start,
                end: range.end,
            })],
        };
        let mut spans = Vec::new();
        let mut visual_x = 0u16;
        let mut line = 0u16;
        let tab_width = doc.tab_width();
        let tab = " ".repeat(tab_width);

        let highlights = highlights.into_iter().map(|event| match event.unwrap() {
            // convert byte offsets to char offset
            helix_core::syntax::HighlightEvent::Source { start, end } => {
                let start = helix_core::graphemes::ensure_grapheme_boundary_next(
                    text,
                    text.byte_to_char(start),
                );
                let end = helix_core::graphemes::ensure_grapheme_boundary_next(
                    text,
                    text.byte_to_char(end),
                );
                helix_core::syntax::HighlightEvent::Source { start, end }
            }
            event => event,
        });

        // let selections = doc.selection(view.id);
        // let primary_idx = selections.primary_index();
        // let selection_scope = theme
        //     .find_scope_index("ui.selection")
        //     .expect("no selection scope found!");

        'outer: for event in highlights {
            match event {
                helix_core::syntax::HighlightEvent::HighlightStart(span) => {
                    spans.push(span);
                }
                helix_core::syntax::HighlightEvent::HighlightEnd => {
                    spans.pop();
                }
                helix_core::syntax::HighlightEvent::Source { start, end } => {
                    // `unwrap_or_else` part is for off-the-end indices of
                    // the rope, to allow cursor highlighting at the end
                    // of the rope.
                    let text = text.get_slice(start..end).unwrap_or_else(|| " ".into());

                    use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                    let style = spans.iter().fold(theme.get("ui.text"), |acc, span| {
                        let style = theme.get(theme.scopes()[span.0].as_str());
                        acc.patch(style)
                    });

                    for grapheme in RopeGraphemes::new(text) {
                        let out_of_bounds = visual_x < view.first_col as u16
                            || visual_x >= viewport.width + view.first_col as u16;

                        if helix_core::LineEnding::from_rope_slice(&grapheme).is_some() {
                            if !out_of_bounds {
                                // we still want to render an empty cell with the style
                                surface.set_string(
                                    viewport.x + visual_x - view.first_col as u16,
                                    viewport.y + line,
                                    " ",
                                    style,
                                );
                            }

                            visual_x = 0;
                            line += 1;

                            // TODO: with proper iter this shouldn't be necessary
                            if line >= viewport.height {
                                break 'outer;
                            }
                        } else {
                            let grapheme = Cow::from(grapheme);

                            let (grapheme, width) = if grapheme == "\t" {
                                // make sure we display tab as appropriate amount of spaces
                                (tab.as_str(), tab_width)
                            } else {
                                // Cow will prevent allocations if span contained in a single slice
                                // which should really be the majority case
                                let width = grapheme_width(&grapheme);
                                (grapheme.as_ref(), width)
                            };

                            if !out_of_bounds {
                                // if we're offscreen just keep going until we hit a new line
                                surface.set_string(
                                    viewport.x + visual_x - view.first_col as u16,
                                    viewport.y + line,
                                    grapheme,
                                    style,
                                );
                            }

                            visual_x = visual_x.saturating_add(width as u16);
                        }
                    }
                }
            }
        }
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

    fn calculate_preview(
        &mut self,
        theme: &helix_view::Theme,
        loader: &helix_core::syntax::Loader,
    ) {
        if let Some((path, range)) = self
            .selection()
            .and_then(|current| (self.preview_fn)(current))
            .and_then(|(path, range)| canonicalize_path(&path).ok().zip(Some(range)))
        {
            let &mut (ref mut doc, ref mut view) = self
                .preview_cache
                .entry((path.clone(), range))
                .or_insert_with(|| {
                    let doc = Document::open(path, None, Some(theme), Some(loader)).unwrap();
                    let view = View::new(doc.id());
                    (doc, view)
                });

            doc.set_selection(view.id, Selection::from(range));
            commands::align_view(doc, view, Align::Center);
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
                self.calculate_preview(&cx.editor.theme, &cx.editor.syn_loader);
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
                self.calculate_preview(&cx.editor.theme, &cx.editor.syn_loader);
            }
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.preview_cache.clear();
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::Replace);
                }
                self.preview_cache.clear();
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
                self.preview_cache.clear();
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.save_filter();
                self.calculate_preview(&cx.editor.theme, &cx.editor.syn_loader);
            }
            _ => {
                if let EventResult::Consumed(_) = self.prompt.handle_event(event, cx) {
                    // TODO: recalculate only if pattern changed
                    self.score();
                    self.calculate_preview(&cx.editor.theme, &cx.editor.syn_loader);
                }
            }
        }

        EventResult::Consumed(None)
    }

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let area = inner_rect(area);

        // -- Render the frame:

        // clear area
        let background = cx.editor.theme.get("ui.background");
        surface.clear_with(area, background);

        use tui::widgets::Widget;
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
        let mut item_width = inner.width;

        if let Some((doc, view)) = self
            .selection()
            .and_then(|current| (self.preview_fn)(current))
            .and_then(|(path, range)| canonicalize_path(&path).ok().zip(Some(range)))
            .and_then(|(path, range)| self.preview_cache.get(&(path, range)))
        {
            item_width = inner.width * 40 / 100;

            for y in inner.top()..inner.bottom() {
                surface
                    .get_mut(inner.x + item_width, y)
                    .set_symbol(borders.vertical)
                    .set_style(sep_style);
            }

            let viewport = Rect::new(
                inner.x + item_width + 1, // 1 for sep
                inner.y,
                inner.width * 60 / 100,
                inner.height,
            );
            // FIXME: last line will not be highlighted because of a -1 in View::last_line
            let mut view = view.clone();
            view.area = viewport;
            self.render_buffer(
                doc,
                &view,
                viewport,
                surface,
                &cx.editor.theme,
                &cx.editor.syn_loader,
            );
        }

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
                item_width as usize,
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
