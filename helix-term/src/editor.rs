use crate::{keymap, Args};
use anyhow::Error;
use crossterm::{
    cursor,
    cursor::position,
    event::{self, read, Event, EventStream, KeyCode, KeyEvent},
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};

use helix_core::{state::coords_at_pos, state::Mode, State};
use smol::prelude::*;
use std::collections::HashMap;
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use tui::backend::CrosstermBackend;
use tui::buffer::Buffer as Surface;
use tui::layout::Rect;
use tui::style::Style;

type Terminal = tui::Terminal<CrosstermBackend<std::io::Stdout>>;

static EX: smol::Executor = smol::Executor::new();

pub struct Editor {
    terminal: Terminal,
    state: Option<State>,
    first_line: u16,
    size: (u16, u16),
    surface: Surface,
    theme: HashMap<&'static str, Style>,
}

impl Editor {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());

        let mut terminal = Terminal::new(backend)?;
        let size = terminal::size().unwrap();
        let area = Rect::new(0, 0, size.0, size.1);

        use tui::style::Color;
        let theme = hashmap! {
            "attribute" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "keyword" => Style::default().fg(Color::Rgb(236, 205, 186)), // almond
            "punctuation" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "punctuation.delimiter" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "operator" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "property" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "variable.parameter" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            // TODO distinguish type from type.builtin?
            "type" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "type.builtin" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "constructor" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "function" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "function.macro" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "comment" => Style::default().fg(Color::Rgb(105, 124, 129)), // sirocco
            "variable.builtin" => Style::default().fg(Color::Rgb(159, 242, 143)), // mint
            "constant" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "constant.builtin" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "string" => Style::default().fg(Color::Rgb(204, 204, 204)), // silver
            "escape" => Style::default().fg(Color::Rgb(239, 186, 93)), // honey
            // used for lifetimes
            "label" => Style::default().fg(Color::Rgb(239, 186, 93)), // honey

            // TODO: diferentiate number builtin
            // TODO: diferentiate doc comment
            // TODO: variable as lilac
            // TODO: mod/use statements as white
            // TODO: mod stuff as chamoise
            // TODO: add "(scoped_identifier) @path" for std::mem::
            //
            // concat (ERROR) @syntax-error and "MISSING ;" selectors for errors

            "module" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
            "variable" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
            "function.builtin" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
        };

        let mut editor = Editor {
            terminal,
            state: None,
            first_line: 0,
            size,
            surface: Surface::empty(area),
            theme,
        };

        if let Some(file) = args.files.pop() {
            editor.open(file)?;
        }

        Ok(editor)
    }

    pub fn open(&mut self, path: PathBuf) -> Result<(), Error> {
        self.state = Some(State::load(path)?);
        Ok(())
    }

    fn render(&mut self) {
        match &self.state {
            Some(state) => {
                let area = Rect::new(0, 0, self.size.0, self.size.1);
                let mut surface = Surface::empty(area);
                let mut stdout = stdout();

                //
                use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

                let highlight_names: Vec<String> = [
                    "attribute",
                    "constant.builtin",
                    "constant",
                    "function.builtin",
                    "function.macro",
                    "function",
                    "keyword",
                    "operator",
                    "property",
                    "punctuation",
                    "comment",
                    "escape",
                    "label",
                    // "punctuation.bracket",
                    "punctuation.delimiter",
                    "string",
                    "string.special",
                    "tag",
                    "type",
                    "type.builtin",
                    "constructor",
                    "variable",
                    "variable.builtin",
                    "variable.parameter",
                    "path",
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

                // TODO: inefficient, should feed chunks.iter() to tree_sitter.parse_with(|offset,
                // pos|)
                let source_code = state.doc.to_string();

                // TODO: cache highlight results
                // TODO: only recalculate when state.doc is actually modified
                let highlights = highlighter
                    .highlight(&config, source_code.as_bytes(), None, |_| None)
                    .unwrap();

                let mut spans = Vec::new();

                let offset = 2;

                let mut visual_x = 0;
                let mut line = 0;

                for event in highlights {
                    match event.unwrap() {
                        HighlightEvent::HighlightStart(span) => {
                            // eprintln!("highlight style started: {:?}", highlight_names[span.0]);
                            spans.push(span);
                        }
                        HighlightEvent::HighlightEnd => {
                            spans.pop();
                            // eprintln!("highlight style ended");
                        }
                        HighlightEvent::Source { start, end } => {
                            // TODO: filter out spans out of viewport for now..

                            let start = state.doc.byte_to_char(start);
                            let end = state.doc.byte_to_char(end);

                            let text = state.doc.slice(start..end);

                            use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                            use tui::style::Color;
                            let style = match spans.first() {
                                Some(span) => self
                                    .theme
                                    .get(highlight_names[span.0].as_str())
                                    .map(|style| *style)
                                    .unwrap_or(Style::default().fg(Color::Rgb(0, 0, 255))),

                                None => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
                                                                                        // None => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
                            };

                            // iterate over range char by char
                            for grapheme in RopeGraphemes::new(&text) {
                                // TODO: track current char_index

                                if grapheme == "\n" {
                                    visual_x = 0;
                                    line += 1;
                                } else {
                                    // Cow will prevent allocations if span contained in a single slice
                                    // which should really be the majority case
                                    let grapheme = std::borrow::Cow::from(grapheme);
                                    let width = grapheme_width(&grapheme) as u16;
                                    surface.set_string(offset + visual_x, line, grapheme, style);

                                    visual_x += width;
                                }
                                // if grapheme == "\t"
                            }
                        }
                    }
                }

                //

                // let lines = state
                //     .doc
                //     .lines_at(self.first_line as usize)
                //     .take(self.size.1 as usize)
                //     .map(|x| x.as_str().unwrap());

                // for (n, line) in lines.enumerate() {
                //     execute!(
                //         stdout,
                //         SetForegroundColor(Color::DarkCyan),
                //         cursor::MoveTo(0, n as u16),
                //         Print((n + 1).to_string())
                //     );

                //     surface.set_string(2, n as u16, line, Style::default());
                //     // execute!(
                //     //     stdout,
                //     //     SetForegroundColor(Color::Reset),
                //     //     cursor::MoveTo(2, n as u16),
                //     //     Print(line)
                //     // );
                // }

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
                //     surface.set_style(area, select);

                //     // TODO: don't highlight next char in append mode
                // }

                // let mode = match state.mode {
                //     Mode::Insert => "INS",
                //     Mode::Normal => "NOR",
                // };

                // execute!(
                //     stdout,
                //     SetForegroundColor(Color::Reset),
                //     cursor::MoveTo(0, self.size.1),
                //     Print(mode)
                // );

                use tui::backend::Backend;
                // // TODO: double buffer and diff here
                self.terminal
                    .backend_mut()
                    .draw(self.surface.diff(&surface).into_iter());
                // swap the buffer
                self.surface = surface;

                // set cursor shape
                match state.mode {
                    Mode::Insert => write!(stdout, "\x1B[6 q"),
                    Mode::Normal => write!(stdout, "\x1B[2 q"),
                };

                // render the cursor
                let pos = state.selection.cursor();
                let coords = coords_at_pos(&state.doc.slice(..), pos);
                execute!(
                    stdout,
                    cursor::MoveTo((coords.1 + 2) as u16, coords.0 as u16)
                );
            }
            None => (),
        }
    }

    pub async fn print_events(&mut self) {
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
                        match state.mode {
                            Mode::Insert => {
                                match event {
                                    KeyEvent {
                                        code: KeyCode::Esc, ..
                                    } => helix_core::commands::normal_mode(state, 1),
                                    KeyEvent {
                                        code: KeyCode::Char(c),
                                        ..
                                    } => helix_core::commands::insert_char(state, c),
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

        self.print_events().await;

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
    use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

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
