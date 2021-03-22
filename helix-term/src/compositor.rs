// Each component declares it's own size constraints and gets fitted based on it's parent.
// Q: how does this work with popups?
// cursive does compositor.screen_mut().add_layer_at(pos::absolute(x, y), <component>)

use crossterm::event::Event;
use helix_core::Position;
use smol::Executor;
use tui::{buffer::Buffer as Surface, layout::Rect};

pub type Callback = Box<dyn FnOnce(&mut Compositor, &mut Editor)>;

// --> EventResult should have a callback that takes a context with methods like .popup(),
// .prompt() etc. That way we can abstract it from the renderer.
// Q: How does this interact with popups where we need to be able to specify the rendering of the
// popup?
// A: It could just take a textarea.
//
// If Compositor was specified in the callback that's then problematic because of

// Cursive-inspired
pub enum EventResult {
    Ignored,
    Consumed(Option<Callback>),
}

use helix_view::{Editor, View};

pub struct Context<'a> {
    pub editor: &'a mut Editor,
    pub executor: &'static smol::Executor<'static>,
    pub scroll: Option<usize>,
}

pub trait Component {
    /// Process input events, return true if handled.
    fn handle_event(&mut self, event: Event, ctx: &mut Context) -> EventResult {
        EventResult::Ignored
    }
    // , args: ()

    /// Should redraw? Useful for saving redraw cycles if we know component didn't change.
    fn should_update(&self) -> bool {
        true
    }

    /// Render the component onto the provided surface.
    fn render(&self, area: Rect, frame: &mut Surface, ctx: &mut Context);

    fn cursor_position(&self, area: Rect, ctx: &Editor) -> Option<Position> {
        None
    }

    /// May be used by the parent component to compute the child area.
    /// viewport is the maximum allowed area, and the child should stay within those bounds.
    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        // TODO: for scrolling, the scroll wrapper should place a size + offset on the Context
        // that way render can use it
        None
    }
}

use anyhow::Error;
use std::io::stdout;
use tui::backend::CrosstermBackend;
type Terminal = crate::terminal::Terminal<CrosstermBackend<std::io::Stdout>>;

pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
    terminal: Terminal,
}

impl Compositor {
    pub fn new() -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)?;
        Ok(Self {
            layers: Vec::new(),
            terminal,
        })
    }

    pub fn size(&self) -> Rect {
        self.terminal.size().expect("couldn't get terminal size")
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal
            .resize(Rect::new(0, 0, width, height))
            .expect("Unable to resize terminal")
    }

    pub fn push(&mut self, layer: Box<dyn Component>) {
        self.layers.push(layer);
    }

    pub fn pop(&mut self) {
        self.layers.pop();
    }

    pub fn handle_event(&mut self, event: Event, cx: &mut Context) -> bool {
        // propagate events through the layers until we either find a layer that consumes it or we
        // run out of layers (event bubbling)
        for layer in self.layers.iter_mut().rev() {
            match layer.handle_event(event, cx) {
                EventResult::Consumed(Some(callback)) => {
                    callback(self, cx.editor);
                    return true;
                }
                EventResult::Consumed(None) => return true,
                EventResult::Ignored => false,
            };
        }
        false
    }

    pub fn render(&mut self, cx: &mut Context) {
        let area = self.size();
        let surface = self.terminal.current_buffer_mut();

        for layer in &self.layers {
            layer.render(area, surface, cx)
        }

        let pos = self
            .cursor_position(area, cx.editor)
            .map(|pos| (pos.col as u16, pos.row as u16));

        self.terminal.draw(pos);
    }

    pub fn cursor_position(&self, area: Rect, editor: &Editor) -> Option<Position> {
        for layer in self.layers.iter().rev() {
            if let Some(pos) = layer.cursor_position(area, editor) {
                return Some(pos);
            }
        }
        None
    }
}
