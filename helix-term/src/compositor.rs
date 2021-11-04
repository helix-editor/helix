// Each component declares it's own size constraints and gets fitted based on it's parent.
// Q: how does this work with popups?
// cursive does compositor.screen_mut().add_layer_at(pos::absolute(x, y), <component>)
use helix_core::Position;
use helix_view::graphics::{CursorKind, Rect};

use crossterm::event::Event;
use tui::buffer::Buffer as Surface;

pub type Callback = Box<dyn FnOnce(&mut Compositor)>;

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

use helix_view::Editor;

use crate::job::Jobs;

pub struct Context<'a> {
    pub editor: &'a mut Editor,
    pub scroll: Option<usize>,
    pub jobs: &'a mut Jobs,
}

pub trait Component: Any + AnyComponent {
    /// Process input events, return true if handled.
    fn handle_event(&mut self, _event: Event, _ctx: &mut Context) -> EventResult {
        EventResult::Ignored
    }
    // , args: ()

    /// Should redraw? Useful for saving redraw cycles if we know component didn't change.
    fn should_update(&self) -> bool {
        true
    }

    /// Render the component onto the provided surface.
    fn render(&mut self, area: Rect, frame: &mut Surface, ctx: &mut Context);

    /// Get cursor position and cursor kind.
    fn cursor(&self, _area: Rect, _ctx: &Editor) -> (Option<Position>, CursorKind) {
        (None, CursorKind::Hidden)
    }

    /// May be used by the parent component to compute the child area.
    /// viewport is the maximum allowed area, and the child should stay within those bounds.
    fn required_size(&mut self, _viewport: (u16, u16)) -> Option<(u16, u16)> {
        // TODO: for scrolling, the scroll wrapper should place a size + offset on the Context
        // that way render can use it
        None
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

use anyhow::Error;
use std::io::stdout;
use tui::backend::{Backend, CrosstermBackend};
type Terminal = tui::terminal::Terminal<CrosstermBackend<std::io::Stdout>>;

pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
    terminal: Terminal,

    pub(crate) last_picker: Option<Box<dyn Component>>,
}

impl Compositor {
    pub fn new() -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            layers: Vec::new(),
            terminal,
            last_picker: None,
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

    pub fn save_cursor(&mut self) {
        if self.terminal.cursor_kind() == CursorKind::Hidden {
            self.terminal
                .backend_mut()
                .show_cursor(CursorKind::Block)
                .ok();
        }
    }

    pub fn load_cursor(&mut self) {
        if self.terminal.cursor_kind() == CursorKind::Hidden {
            self.terminal.backend_mut().hide_cursor().ok();
        }
    }

    pub fn push(&mut self, mut layer: Box<dyn Component>) {
        let size = self.size();
        // trigger required_size on init
        layer.required_size((size.width, size.height));
        self.layers.push(layer);
    }

    pub fn pop(&mut self) -> Option<Box<dyn Component>> {
        self.layers.pop()
    }

    pub fn handle_event(&mut self, event: Event, cx: &mut Context) -> bool {
        // propagate events through the layers until we either find a layer that consumes it or we
        // run out of layers (event bubbling)
        for layer in self.layers.iter_mut().rev() {
            match layer.handle_event(event, cx) {
                EventResult::Consumed(Some(callback)) => {
                    callback(self);
                    return true;
                }
                EventResult::Consumed(None) => return true,
                EventResult::Ignored => false,
            };
        }
        false
    }

    pub fn render(&mut self, cx: &mut Context) {
        self.terminal
            .autoresize()
            .expect("Unable to determine terminal size");

        // TODO: need to recalculate view tree if necessary

        let surface = self.terminal.current_buffer_mut();

        let area = *surface.area();

        for layer in &mut self.layers {
            layer.render(area, surface, cx);
        }

        let (pos, kind) = self.cursor(area, cx.editor);
        let pos = pos.map(|pos| (pos.col as u16, pos.row as u16));

        self.terminal.draw(pos, kind).unwrap();
    }

    pub fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        for layer in self.layers.iter().rev() {
            if let (Some(pos), kind) = layer.cursor(area, editor) {
                return (Some(pos), kind);
            }
        }
        (None, CursorKind::Hidden)
    }

    pub fn has_component(&self, type_name: &str) -> bool {
        self.layers
            .iter()
            .any(|component| component.type_name() == type_name)
    }

    pub fn find(&mut self, type_name: &str) -> Option<&mut dyn Component> {
        self.layers
            .iter_mut()
            .find(|component| component.type_name() == type_name)
            .map(|component| component.as_mut())
    }
}

// View casting, taken straight from Cursive

use std::any::Any;

/// A view that can be downcasted to its concrete type.
///
/// This trait is automatically implemented for any `T: Component`.
pub trait AnyComponent {
    /// Downcast self to a `Any`.
    fn as_any(&self) -> &dyn Any;

    /// Downcast self to a mutable `Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Returns a boxed any from a boxed self.
    ///
    /// Can be used before `Box::downcast()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use helix_term::{ui::Text, compositor::Component};
    /// let boxed: Box<dyn Component> = Box::new(Text::new("text".to_string()));
    /// let text: Box<Text> = boxed.as_boxed_any().downcast().unwrap();
    /// ```
    fn as_boxed_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Component> AnyComponent for T {
    /// Downcast self to a `Any`.
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Downcast self to a mutable `Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl dyn AnyComponent {
    /// Attempts to downcast `self` to a concrete type.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    /// Attempts to downcast `self` to a concrete type.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }

    /// Attempts to downcast `Box<Self>` to a concrete type.
    pub fn downcast<T: Any>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        // Do the check here + unwrap, so the error
        // value is `Self` and not `dyn Any`.
        if self.as_any().is::<T>() {
            Ok(self.as_boxed_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Checks if this view is of type `T`.
    pub fn is<T: Any>(&mut self) -> bool {
        self.as_any().is::<T>()
    }
}
