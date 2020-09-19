use crate::{keymap, theme::Theme, Args};
use helix_core::{
    state::coords_at_pos,
    state::Mode,
    syntax::{HighlightConfiguration, HighlightEvent, Highlighter},
    State,
};

use std::{
    io::{self, stdout, Write},
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

type Terminal = tui::Terminal<CrosstermBackend<std::io::Stdout>>;

static EX: smol::Executor = smol::Executor::new();

pub struct Editor {
    terminal: Terminal,
    state: Option<State>,
    first_line: u16,
    size: (u16, u16),
    surface: Surface,
    cache: Surface,
    theme: Theme,
}

impl Editor {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());

        let mut terminal = Terminal::new(backend)?;
        let size = terminal::size().unwrap();
        let area = Rect::new(0, 0, size.0, size.1);
        let theme = Theme::default();

        let mut editor = Editor {
            terminal,
            state: None,
            first_line: 0,
            size,
            surface: Surface::empty(area),
            cache: Surface::empty(area),
            theme,
            // TODO; move to state
        };

        if let Some(file) = args.files.pop() {
            editor.open(file)?;
        }

        Ok(editor)
    }

    pub fn open(&mut self, path: PathBuf) -> Result<(), Error> {
        let mut state = State::load(path)?;
        state
            .syntax
            .as_mut()
            .unwrap()
            .configure(self.theme.scopes());
        self.state = Some(state);
        Ok(())
    }

    fn render(&mut self) {
        use tui::backend::Backend;
        use tui::style::Color;
        // TODO: ideally not mut but highlights require it because of cursor cache
        match &mut self.state {
            Some(state) => {
                let area = Rect::new(0, 0, self.size.0, self.size.1);
                let mut stdout = stdout();
                self.surface.reset(); // reset is faster than allocating new empty surface

                //  clear with background color
                self.surface
                    .set_style(area, self.theme.get("ui.background"));

                let offset = 5 + 1; // 5 linenr + 1 gutter
                let viewport = Rect::new(offset, 0, self.size.0, self.size.1 - 1); // - 1 for statusline

                // TODO: inefficient, should feed chunks.iter() to tree_sitter.parse_with(|offset, pos|)
                let source_code = state.doc().to_string();

                let last_line = std::cmp::min(
                    (self.first_line + viewport.height - 1) as usize,
                    state.doc().len_lines() - 1,
                );

                let range = {
                    // calculate viewport byte ranges
                    let start = state.doc().line_to_byte(self.first_line.into());
                    let end = state.doc().line_to_byte(last_line)
                        + state.doc().line(last_line).len_bytes();

                    start..end
                };

                // TODO: range doesn't actually restrict source, just highlight range

                // TODO: cache highlight results
                // TODO: only recalculate when state.doc is actually modified
                let highlights: Vec<_> = state
                    .syntax
                    .as_mut()
                    .unwrap()
                    .highlight_iter(source_code.as_bytes(), Some(range), None, |_| None)
                    .unwrap()
                    .collect(); // TODO: we collect here to avoid double borrow, fix later

                let mut spans = Vec::new();

                let mut visual_x = 0;
                let mut line = 0u16;

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

                            let start = state.doc().byte_to_char(start);
                            let end = state.doc().byte_to_char(end);

                            let text = state.doc().slice(start..end);

                            use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                            let style = match spans.first() {
                                Some(span) => self.theme.get(self.theme.scopes()[span.0].as_str()),
                                None => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
                            };

                            // TODO: we could render the text to a surface, then cache that, that
                            // way if only the selection/cursor changes we can copy from cache
                            // and paint the new cursor.

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
                                } else {
                                    // Cow will prevent allocations if span contained in a single slice
                                    // which should really be the majority case
                                    let grapheme = std::borrow::Cow::from(grapheme);
                                    let width = grapheme_width(&grapheme) as u16;
                                    self.surface.set_string(
                                        offset + visual_x,
                                        line,
                                        grapheme,
                                        style,
                                    );

                                    visual_x += width;
                                }
                                // if grapheme == "\t"
                            }
                        }
                    }
                }

                let mut line = 0;
                let style = self.theme.get("ui.linenr");
                for i in self.first_line..(last_line as u16) {
                    self.surface
                        .set_stringn(0, line, format!("{:>5}", i + 1), 5, style); // lavender
                    line += 1;
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
                let mode = match state.mode() {
                    Mode::Insert => "INS",
                    Mode::Normal => "NOR",
                };
                self.surface.set_style(
                    Rect::new(0, self.size.1 - 1, self.size.0, 1),
                    self.theme.get("ui.statusline"),
                );
                // TODO: unfocused one with different color
                let text_color = Style::default().fg(Color::Rgb(219, 191, 239)); // lilac
                self.surface
                    .set_string(1, self.size.1 - 1, mode, text_color);
                if let Some(path) = state.path() {
                    self.surface
                        .set_string(6, self.size.1 - 1, path.to_string_lossy(), text_color);
                }

                self.terminal
                    .backend_mut()
                    .draw(self.cache.diff(&self.surface).into_iter());
                // swap the buffer
                std::mem::swap(&mut self.surface, &mut self.cache);

                // set cursor shape
                match state.mode() {
                    Mode::Insert => write!(stdout, "\x1B[6 q"),
                    Mode::Normal => write!(stdout, "\x1B[2 q"),
                };

                // render the cursor
                let pos = state.selection().cursor();
                let coords = coords_at_pos(&state.doc().slice(..), pos);
                execute!(
                    stdout,
                    cursor::MoveTo(
                        coords.col as u16 + viewport.x,
                        coords.row as u16 - self.first_line + viewport.y,
                    )
                );
            }
            None => (),
        }
    }

    pub async fn event_loop(&mut self) {
        let mut reader = EventStream::new();
        let keymap = keymap::default();

        self.render();

        loop {
            // Handle key events
            let mut event = reader.next().await;
            match event {
                // TODO: handle resize events
                Some(Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }))) => {
                    break;
                }
                Some(Ok(Event::Key(event))) => {
                    if let Some(state) = &mut self.state {
                        match state.mode() {
                            Mode::Insert => {
                                match event {
                                    KeyEvent {
                                        code: KeyCode::Esc, ..
                                    } => helix_core::commands::normal_mode(state, 1),
                                    KeyEvent {
                                        code: KeyCode::Backspace,
                                        ..
                                    } => helix_core::commands::delete_char_backward(state, 1),
                                    KeyEvent {
                                        code: KeyCode::Delete,
                                        ..
                                    } => helix_core::commands::delete_char_forward(state, 1),
                                    KeyEvent {
                                        code: KeyCode::Char(c),
                                        ..
                                    } => helix_core::commands::insert_char(state, c),
                                    KeyEvent {
                                        code: KeyCode::Enter,
                                        ..
                                    } => helix_core::commands::insert_char(state, '\n'),
                                    _ => (), // skip
                                }
                                self.render();
                            }
                            Mode::Normal => {
                                // TODO: handle modes and sequences (`gg`)
                                if let Some(command) = keymap.get(&event) {
                                    // TODO: handle count other than 1
                                    command(state, 1);

                                    self.render();
                                }
                            }
                        }
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
//
// let visual_x = 0;
// let line = ?;
// for span in spans {
// start(scope) => scopes.push(scope)
//  span =>
//      let text = rope.slice(span.start..span.end);
//      let style = calculate_style(scopes);
//      for each grapheme in text.graphemes() {
//          // if newline += lines, continue
//
//          if state.selection.ranges().any(|range| range.contains(char_index)) {
//              if exactly on cursor {
//              }
//              if on primary cursor? {
//              }
//              modify style temporarily
//          }
//
//          // if in bounds
//
//          // if tab, draw tab width
//          // draw(visual_x, line, grapheme, style)
//          // increment visual_x by grapheme_width(grapheme)
//          // increment char_index by grapheme.len_chars()
//      }
//  end => scopes.pop()
// }
#[test]
fn test_parser() {
    use helix_core::syntax::{HighlightConfiguration, HighlightEvent, Highlighter};

    let source_code = include_str!("../test.rs");

    let highlight_names: Vec<String> = [
        "attribute",
        "constant",
        "function.builtin",
        "function",
        "keyword",
        "operator",
        "property",
        "punctuation",
        "punctuation.bracket",
        "punctuation.delimiter",
        "string",
        "string.special",
        "tag",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
        "variable.parameter",
    ]
    .iter()
    .cloned()
    .map(String::from)
    .collect();

    let language = helix_syntax::get_language(&helix_syntax::LANG::Rust);
    // let mut parser = tree_sitter::Parser::new();
    // parser.set_language(language).unwrap();
    // let tree = parser.parse(source_code, None).unwrap();

    let mut highlighter = Highlighter::new();

    let mut config = HighlightConfiguration::new(
        language,
        &std::fs::read_to_string(
            "../helix-syntax/languages/tree-sitter-rust/queries/highlights.scm",
        )
        .unwrap(),
        &std::fs::read_to_string(
            "../helix-syntax/languages/tree-sitter-rust/queries/injections.scm",
        )
        .unwrap(),
        "", // locals.scm
    )
    .unwrap();

    config.configure(&highlight_names);

    let highlights = highlighter
        .highlight(&config, source_code.as_bytes(), None, |_| None)
        .unwrap();

    for event in highlights {
        match event.unwrap() {
            HighlightEvent::Source { start, end } => {
                eprintln!("source: {}-{}", start, end);
                // iterate over range char by char
            }
            HighlightEvent::HighlightStart(s) => {
                eprintln!("highlight style started: {:?}", highlight_names[s.0]);
                // store/push highlight styles
            }
            HighlightEvent::HighlightEnd => {
                eprintln!("highlight style ended");
                // pop highlight styles
            }
        }
    }
}
