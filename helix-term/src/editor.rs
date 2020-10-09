use clap::ArgMatches as Args;
use helix_core::{indent::TAB_WIDTH, state::Mode, syntax::HighlightEvent, Position, Range, State};
use helix_view::{commands, keymap, prompt::Prompt, View};

use std::{
    borrow::Cow,
    io::{self, stdout, Stdout, Write},
    path::PathBuf,
    time::Duration,
};

use smol::prelude::*;

use anyhow::Error;

use crossterm::{
    cursor,
    cursor::position,
    event::{self, read, Event, EventStream, KeyCode, KeyEvent},
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};

use tui::{backend::CrosstermBackend, buffer::Buffer as Surface, layout::Rect, style::Style};

const OFFSET: u16 = 6; // 5 linenr + 1 gutter

type Terminal = tui::Terminal<CrosstermBackend<std::io::Stdout>>;

static EX: smol::Executor = smol::Executor::new();

pub struct Editor {
    terminal: Terminal,
    view: Option<View>,
    size: (u16, u16),
    surface: Surface,
    cache: Surface,
    prompt: Prompt,
}

impl Editor {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());

        let mut terminal = Terminal::new(backend)?;
        let size = terminal::size().unwrap();
        let area = Rect::new(0, 0, size.0, size.1);
        let prompt = Prompt::new();

        let mut editor = Editor {
            terminal,
            view: None,
            size,
            surface: Surface::empty(area),
            cache: Surface::empty(area),
            // TODO; move to state
            prompt,
        };

        if let Some(file) = args.values_of_t::<PathBuf>("files").unwrap().pop() {
            editor.open(file)?;
        }

        Ok(editor)
    }

    pub fn open(&mut self, path: PathBuf) -> Result<(), Error> {
        self.view = Some(View::open(path, self.size)?);
        Ok(())
    }

    fn render(&mut self) {
        use tui::backend::Backend;
        use tui::style::Color;
        // TODO: ideally not mut but highlights require it because of cursor cache
        let viewport = Rect::new(OFFSET, 0, self.size.0, self.size.1 - 1); // - 1 for statusline
        let text_color: Style = Style::default().fg(Color::Rgb(219, 191, 239)); // lilac

        self.render_view(viewport);

        self.render_prompt(text_color);

        self.render_cursor(viewport, text_color);
    }

    pub fn render_view(&mut self, viewport: Rect) {
        use tui::style::Color;
        let area = Rect::new(0, 0, self.size.0, self.size.1);
        let mut view: &mut View = self.view.as_mut().unwrap();
        self.surface.reset(); // reset is faster than allocating new empty surface

        //  clear with background color
        self.surface
            .set_style(area, view.theme.get("ui.background"));

        // TODO: inefficient, should feed chunks.iter() to tree_sitter.parse_with(|offset, pos|)
        let source_code = view.state.doc().to_string();

        let last_line = view.last_line();

        let range = {
            // calculate viewport byte ranges
            let start = view.state.doc().line_to_byte(view.first_line);
            let end = view.state.doc().line_to_byte(last_line)
                + view.state.doc().line(last_line).len_bytes();

            start..end
        };

        // TODO: range doesn't actually restrict source, just highlight range
        // TODO: cache highlight results
        // TODO: only recalculate when state.doc is actually modified
        let highlights: Vec<_> = match view.state.syntax.as_mut() {
            Some(syntax) => {
                syntax
                    .highlight_iter(source_code.as_bytes(), Some(range), None, |_| None)
                    .unwrap()
                    .collect() // TODO: we collect here to avoid double borrow, fix later
            }
            None => vec![Ok(HighlightEvent::Source {
                start: range.start,
                end: range.end,
            })],
        };
        let mut spans = Vec::new();
        let mut visual_x = 0;
        let mut line = 0u16;
        let visible_selections: Vec<Range> = view
            .state
            .selection()
            .ranges()
            .iter()
            // TODO: limit selection to one in viewport
            // .filter(|range| !range.is_empty()) // && range.overlaps(&Range::new(start, end + 1))
            .copied()
            .collect();

        'outer: for event in highlights {
            match event.unwrap() {
                HighlightEvent::HighlightStart(span) => {
                    spans.push(span);
                }
                HighlightEvent::HighlightEnd => {
                    spans.pop();
                }
                HighlightEvent::Source { start, end } => {
                    // TODO: filter out spans out of viewport for now..

                    let start = view.state.doc().byte_to_char(start);
                    let end = view.state.doc().byte_to_char(end); // <-- index 744, len 743

                    let text = view.state.doc().slice(start..end);

                    use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                    let style = match spans.first() {
                        Some(span) => view.theme.get(view.theme.scopes()[span.0].as_str()),
                        None => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
                    };

                    // TODO: we could render the text to a surface, then cache that, that
                    // way if only the selection/cursor changes we can copy from cache
                    // and paint the new cursor.

                    let mut char_index = start;

                    // iterate over range char by char
                    for grapheme in RopeGraphemes::new(&text) {
                        // TODO: track current char_index

                        if grapheme == "\n" {
                            visual_x = 0;
                            line += 1;

                            // TODO: with proper iter this shouldn't be necessary
                            if line >= viewport.height {
                                break 'outer;
                            }
                        } else if grapheme == "\t" {
                            visual_x += (TAB_WIDTH as u16);
                        } else {
                            // Cow will prevent allocations if span contained in a single slice
                            // which should really be the majority case
                            let grapheme = Cow::from(grapheme);
                            let width = grapheme_width(&grapheme) as u16;

                            // TODO: this should really happen as an after pass
                            let style = if visible_selections
                                .iter()
                                .any(|range| range.contains(char_index))
                            {
                                // cedar
                                style.clone().bg(Color::Rgb(128, 47, 0))
                            } else {
                                style
                            };

                            let style = if visible_selections
                                .iter()
                                .any(|range| range.head == char_index)
                            {
                                style.clone().bg(Color::Rgb(255, 255, 255))
                            } else {
                                style
                            };

                            // TODO: paint cursor heads except primary

                            self.surface
                                .set_string(OFFSET + visual_x, line, grapheme, style);

                            visual_x += width;
                        }
                        // if grapheme == "\t"

                        char_index += 1;
                    }
                }
            }
        }
    }

    pub fn render_prompt(&mut self, text_color: Style) {
        use tui::backend::Backend;
        let view = self.view.as_ref().unwrap();
        let style: Style = view.theme.get("ui.linenr");
        let last_line = view.last_line();
        for (i, line) in (view.first_line..last_line).enumerate() {
            self.surface
                .set_stringn(0, i as u16, format!("{:>5}", line + 1), 5, style);
            // lavender
        }

        // let lines = state
        //     .doc
        //     .lines_at(self.first_line as usize)
        //     .take(self.size.1 as usize)
        //     .map(|x| x.as_str().unwrap());

        // // iterate over selections and render them
        // let select = Style::default().bg(tui::style::Color::LightBlue);
        // let text = state.doc.slice(..);
        // for range in state.selection.ranges() {
        //     // get terminal coords for x,y for each range pos
        //     // TODO: this won't work with multiline
        //     let (y1, x1) = coords_at_pos(&text, range.from());
        //     let (y2, x2) = coords_at_pos(&text, range.to());
        //     let area = Rect::new(
        //         (x1 + 2) as u16,
        //         y1 as u16,
        //         (x2 - x1 + 1) as u16,
        //         (y2 - y1 + 1) as u16,
        //     );
        //     self.surface.set_style(area, select);

        //     // TODO: don't highlight next char in append mode
        // }

        // statusline
        let mode = match view.state.mode() {
            Mode::Insert => "INS",
            Mode::Normal => "NOR",
            Mode::Goto => "GOTO",
            Mode::Command => ":",
        };
        self.surface.set_style(
            Rect::new(0, self.size.1 - 1, self.size.0, 1),
            view.theme.get("ui.statusline"),
        );
        // render buffer text
        let buffer_string = &self.prompt.buffer;
        self.surface
            .set_string(2, self.size.1 - 1, buffer_string, text_color);

        self.surface
            .set_string(1, self.size.1 - 1, mode, text_color);

        self.terminal
            .backend_mut()
            .draw(self.cache.diff(&self.surface).into_iter());
        // swap the buffer
        std::mem::swap(&mut self.surface, &mut self.cache);
    }

    pub fn render_cursor(&mut self, viewport: Rect, text_color: Style) {
        let mut pos: Position;
        let view = self.view.as_ref().unwrap();
        let mut stdout = stdout();
        match view.state.mode() {
            Mode::Insert => write!(stdout, "\x1B[6 q"),
            mode => write!(stdout, "\x1B[2 q"),
        };
        if view.state.mode() == Mode::Command {
            pos = Position::new(self.size.0 as usize, 2 + self.prompt.buffer.len());
        } else {
            if let Some(path) = view.state.path() {
                self.surface
                    .set_string(6, self.size.1 - 1, path.to_string_lossy(), text_color);
            }

            let cursor = view.state.selection().cursor();

            pos = view
                .screen_coords_at_pos(&view.state.doc().slice(..), cursor)
                .expect("Cursor is out of bounds.");
            pos.col += viewport.x as usize;
            pos.row += viewport.y as usize;
        }

        execute!(stdout, cursor::MoveTo(pos.col as u16, pos.row as u16));
    }

    pub async fn event_loop(&mut self) {
        let mut reader = EventStream::new();
        let keymap = keymap::default();

        self.render();

        loop {
            // Handle key events
            let mut event = reader.next().await;
            match event {
                Some(Ok(Event::Resize(width, height))) => {
                    self.size = (width, height);
                    let area = Rect::new(0, 0, width, height);
                    self.surface = Surface::empty(area);
                    self.cache = Surface::empty(area);

                    // TODO: simplistic ensure cursor in view for now
                    if let Some(view) = &mut self.view {
                        view.size = self.size;
                        view.ensure_cursor_in_view()
                    };

                    self.render();
                }
                Some(Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }))) => {
                    break;
                }

                Some(Ok(Event::Key(event))) => {
                    // TODO: sequences (`gg`)
                    // TODO: handle count other than 1
                    if let Some(view) = &mut self.view {
                        let keys = vec![event];
                        match view.state.mode() {
                            Mode::Insert => {
                                if let Some(command) = keymap[&Mode::Insert].get(&keys) {
                                    command(view, 1);
                                } else if let KeyEvent {
                                    code: KeyCode::Char(c),
                                    ..
                                } = event
                                {
                                    commands::insert::insert_char(view, c);
                                }
                                view.ensure_cursor_in_view();
                            }
                            Mode::Command => {
                                if let Some(command) = keymap[&Mode::Command].get(&keys) {
                                    command(view, 1);
                                } else if let KeyEvent {
                                    code: KeyCode::Char(c),
                                    ..
                                } = event
                                {
                                    commands::insert_char_prompt(&mut self.prompt, c)
                                }
                            }
                            mode => {
                                if let Some(command) = keymap[&mode].get(&keys) {
                                    command(view, 1);

                                    // TODO: simplistic ensure cursor in view for now
                                    view.ensure_cursor_in_view();
                                }
                            }
                        }
                        self.render();
                    }
                }
                Some(Ok(_)) => {
                    // unhandled event
                }
                Some(Err(x)) => panic!(x),
                None => break,
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        enable_raw_mode()?;

        let mut stdout = stdout();

        execute!(stdout, terminal::EnterAlternateScreen)?;

        // Exit the alternate screen and disable raw mode before panicking
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            execute!(std::io::stdout(), terminal::LeaveAlternateScreen);
            disable_raw_mode();
            hook(info);
        }));

        self.event_loop().await;

        // reset cursor shape
        write!(stdout, "\x1B[2 q");

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        disable_raw_mode()?;

        Ok(())
    }
}

// TODO: language configs:
// tabSize, fileExtension etc, mapping to tree sitter parser
// themes:
// map tree sitter highlights to color values
//
// TODO: expand highlight thing so we're able to render only viewport range
// TODO: async: maybe pre-cache scopes as empty so we render all graphemes initially as regular
////text until calc finishes
// TODO: scope matching: biggest union match? [string] & [html, string], [string, html] & [ string, html]
// can do this by sorting our theme matches based on array len (longest first) then stopping at the
// first rule that matches (rule.all(|scope| scopes.contains(scope)))
