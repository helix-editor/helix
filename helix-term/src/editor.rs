#![allow(unused)]
use anyhow::Error;
use termwiz::caps::Capabilities;
use termwiz::cell::AttributeChange;
use termwiz::color::{AnsiColor, ColorAttribute, RgbColor};
use termwiz::input::*;
use termwiz::surface::Change;
use termwiz::terminal::{buffered::BufferedTerminal, SystemTerminal, Terminal};
use termwiz::widgets::*;

use crate::Args;

use std::{env, path::PathBuf};

use helix_core::Buffer;

/// This is a widget for our application
pub struct MainScreen {}

impl MainScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl Widget for MainScreen {
    fn process_event(&mut self, event: &WidgetEvent, _args: &mut UpdateArgs) -> bool {
        true // handled it all
    }

    /// Draw ourselves into the surface provided by RenderArgs
    fn render(&mut self, args: &mut RenderArgs) {
        // args.surface.add_change(Change::ClearScreen(
        //     ColorAttribute::TrueColorWithPaletteFallback(
        //         RgbColor::new(0x31, 0x1B, 0x92),
        //         AnsiColor::Black.into(),
        //     ),
        // ));
        // args.surface
        //     .add_change(Change::Attribute(AttributeChange::Foreground(
        //         ColorAttribute::TrueColorWithPaletteFallback(
        //             RgbColor::new(0xB3, 0x88, 0xFF),
        //             AnsiColor::Purple.into(),
        //         ),
        //     )));
    }

    fn get_size_constraints(&self) -> layout::Constraints {
        let mut constraints = layout::Constraints::default();
        constraints.child_orientation = layout::ChildOrientation::Vertical;
        constraints
    }
}

pub struct BufferComponent<'a> {
    text: String,
    buffer: &'a mut Buffer,

    first_line: usize,
}

impl<'a> BufferComponent<'a> {
    /// Initialize the widget with the input text
    pub fn new(buffer: &'a mut Buffer) -> Self {
        Self {
            buffer,
            text: String::new(),

            first_line: 0,
        }
    }
}

impl<'a> Widget for BufferComponent<'a> {
    fn process_event(&mut self, event: &WidgetEvent, _args: &mut UpdateArgs) -> bool {
        match event {
            WidgetEvent::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::Char('k'),
                ..
            })) => {
                self.first_line = self.first_line.saturating_sub(1);
            }
            WidgetEvent::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::Char('j'),
                ..
            })) => {
                self.first_line = self.first_line.saturating_add(1);
            }
            WidgetEvent::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::Enter,
                ..
            })) => {
                self.text.push_str("\r\n");
            }
            WidgetEvent::Input(InputEvent::Paste(s)) => {
                self.text.push_str(&s);
            }
            _ => {}
        }

        true // handled it all
    }

    /// Draw ourselves into the surface provided by RenderArgs
    fn render(&mut self, args: &mut RenderArgs) {
        args.surface
            .add_change(Change::ClearScreen(ColorAttribute::Default));

        // args.surface
        //     .add_change(Change::Attribute(AttributeChange::Foreground(
        //         ColorAttribute::TrueColorWithPaletteFallback(
        //             RgbColor::new(0x11, 0x00, 0xFF),
        //             AnsiColor::Purple.into(),
        //         ),
        //     )));
        let (_width, height) = args.surface.dimensions();

        for line in self.buffer.contents.lines_at(self.first_line).take(height) {
            args.surface
                .add_change(unsafe { String::from_utf8_unchecked(line.bytes().collect()) });
            args.surface.add_change("\r");
        }
        // args.surface
        //     .add_change(format!("🤷 surface size is {:?}\r\n", dims));
        // args.surface.add_change(self.text.clone());

        // Place the cursor at the end of the text.
        // A more advanced text editing widget would manage the
        // cursor position differently.
        *args.cursor = CursorShapeAndPosition {
            coords: args.surface.cursor_position().into(),
            shape: termwiz::surface::CursorShape::SteadyBar,
            ..Default::default()
        };
    }

    fn get_size_constraints(&self) -> layout::Constraints {
        let mut c = layout::Constraints::default();
        c.set_valign(layout::VerticalAlignment::Top);
        c
    }
}

pub struct StatusLine {}

impl StatusLine {
    pub fn new() -> Self {
        StatusLine {}
    }
}
impl Widget for StatusLine {
    fn process_event(&mut self, event: &WidgetEvent, _args: &mut UpdateArgs) -> bool {
        true
    }

    fn render(&mut self, args: &mut RenderArgs) {
        args.surface.add_change(Change::ClearScreen(
            ColorAttribute::TrueColorWithPaletteFallback(
                RgbColor::new(0xFF, 0xFF, 0xFF),
                AnsiColor::Black.into(),
            ),
        ));
        args.surface
            .add_change(Change::Attribute(AttributeChange::Foreground(
                ColorAttribute::TrueColorWithPaletteFallback(
                    RgbColor::new(0x00, 0x00, 0x00),
                    AnsiColor::Black.into(),
                ),
            )));

        args.surface.add_change(" helix");
    }

    fn get_size_constraints(&self) -> layout::Constraints {
        *layout::Constraints::default()
            .set_fixed_height(1)
            .set_valign(layout::VerticalAlignment::Bottom)
    }
}

pub struct Editor {
    terminal: BufferedTerminal<SystemTerminal>,

    buffer: Option<Buffer>,
}

impl Editor {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        // Create a terminal
        let caps = Capabilities::new_from_env()?;
        let mut terminal = BufferedTerminal::new(SystemTerminal::new(caps)?)?;

        let mut editor = Editor {
            terminal,
            buffer: None,
        };

        if let Some(file) = args.files.pop() {
            editor.open(file)?;
        }

        Ok(editor)
    }

    pub fn open(&mut self, path: PathBuf) -> Result<(), Error> {
        let buffer = Buffer::load(path)?;
        self.buffer = Some(buffer);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Error> {
        // Start with an empty string; typing into the app will
        // update this string.
        let mut typed_text = String::new();

        {
            let buf = &mut self.terminal;
            // Put the terminal in raw mode + alternate screen
            buf.terminal().enter_alternate_screen()?;
            buf.terminal().set_raw_mode()?;

            // Set up the UI
            let mut ui = Ui::new();

            let root_id = ui.set_root(MainScreen::new());
            let buffer_id =
                ui.add_child(root_id, BufferComponent::new(self.buffer.as_mut().unwrap()));
            // let root_id = ui.set_root(Buffer::new(&mut typed_text));
            ui.add_child(root_id, StatusLine::new());
            ui.set_focus(buffer_id);

            loop {
                ui.process_event_queue()?;

                // After updating and processing all of the widgets, compose them
                // and render them to the screen.
                if ui.render_to_screen(buf)? {
                    // We have more events to process immediately; don't block waiting
                    // for input below, but jump to the top of the loop to re-run the
                    // updates.
                    continue;
                }
                // Compute an optimized delta to apply to the terminal and display it
                buf.flush()?;

                // Wait for user input
                match buf.terminal().poll_input(None) {
                    Ok(Some(InputEvent::Resized { rows, cols })) => {
                        // FIXME: this is working around a bug where we don't realize
                        // that we should redraw everything on resize in BufferedTerminal.
                        buf.add_change(Change::ClearScreen(Default::default()));
                        buf.resize(cols, rows);
                    }
                    Ok(Some(input)) => match input {
                        InputEvent::Key(KeyEvent {
                            key: KeyCode::Escape,
                            ..
                        }) => {
                            // Quit the app when escape is pressed
                            break;
                        }
                        input @ _ => {
                            // Feed input into the Ui
                            ui.queue_event(WidgetEvent::Input(input));
                        }
                    },
                    Ok(None) => {}
                    Err(e) => {
                        print!("{:?}\r\n", e);
                        break;
                    }
                }
            }
        }

        // After we've stopped the full screen raw terminal,
        // print out the final edited value of the input text.
        println!("The text you entered: {}", typed_text);
        Ok(())
    }

    pub fn render(&self) {
        // create a new window sized surface
        // paint all components
        // diff vs last frame, swap
        // paint diff
    }
}
