// Features:
// Tracks currently focused component which receives all input
// Event loop is external as opposed to cursive-rs
// Calls render on the component and translates screen coords to local component coords
//
// TODO:
// Q: where is the Application state stored? do we store it into an external static var?
// A: probably makes sense to initialize the editor into a `static Lazy<>` global var.
//
// Q: how do we composit nested structures? There should be sub-components/views
//
// Each component declares it's own size constraints and gets fitted based on it's parent.
// Q: how does this work with popups?
// cursive does compositor.screen_mut().add_layer_at(pos::absolute(x, y), <component>)

use crossterm::event::Event;
use helix_core::Position;
use smol::Executor;
use tui::buffer::Buffer as Surface;
use tui::layout::Rect;

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
// shared with commands.rs
pub struct Context<'a> {
    pub editor: &'a mut Editor,
    pub executor: &'static smol::Executor<'static>,
}

pub trait Component {
    /// Process input events, return true if handled.
    fn handle_event(&mut self, event: Event, ctx: &mut Context) -> EventResult;
    // , args: ()

    /// Should redraw? Useful for saving redraw cycles if we know component didn't change.
    fn should_update(&self) -> bool {
        true
    }

    fn render(&self, area: Rect, frame: &mut Surface, ctx: &mut Context);

    fn cursor_position(&self, area: Rect, ctx: &mut Context) -> Option<Position> {
        None
    }
}

// struct Editor { };

// For v1:
// Child views are something each view needs to handle on it's own for now, positioning and sizing
// options, focus tracking. In practice this is simple: we only will need special solving for
// splits etc

// impl Editor {
//     fn render(&mut self, surface: &mut Surface, args: ()) {
//         // compute x, y, w, h rects for sub-views!
//         // get surface area
//         // get constraints for textarea, statusbar
//         // -> cassowary-rs

//         // first render textarea
//         // then render statusbar
//     }
// }

// usecases to consider:
// - a single view with subviews (textarea + statusbar)
// - a popup panel / dialog with it's own interactions
// - an autocomplete popup that doesn't change focus

//fn main() {
//    let root = Editor::new();
//    let compositor = Compositor::new();

//    compositor.push(root);

//    // pos: clip to bottom of screen
//    compositor.push_at(pos, Prompt::new(
//        ":",
//        (),
//        |input: &str| match input {}
//    )); // TODO: this Prompt needs to somehow call compositor.pop() on close, but it can't refer to parent
//    // Cursive solves this by allowing to return a special result on process_event
//    // that's either Ignore | Consumed(Opt<C>) where C: fn (Compositor) -> ()

//    // TODO: solve popup focus: we want to push autocomplete popups on top of the current layer
//    // but retain the focus where it was. The popup will also need to update as we type into the
//    // textarea. It should also capture certain input, such as tab presses etc
//    //
//    // 1) This could be faked by the top layer pushing down edits into the previous layer.
//    // 2) Alternatively,
//}

pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
}

impl Compositor {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn push(&mut self, layer: Box<dyn Component>) {
        self.layers.push(layer);
    }

    pub fn pop(&mut self) {
        self.layers.pop();
    }

    pub fn handle_event(&mut self, event: Event, cx: &mut Context) -> bool {
        // TODO: custom focus
        if let Some(layer) = self.layers.last_mut() {
            return match layer.handle_event(event, cx) {
                EventResult::Consumed(Some(callback)) => {
                    callback(self, cx.editor);
                    true
                }
                EventResult::Consumed(None) => true,
                EventResult::Ignored => false,
            };
        }
        false
    }

    pub fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        for layer in &self.layers {
            layer.render(area, surface, cx)
        }
    }

    pub fn cursor_position(&self, area: Rect, cx: &mut Context) -> Position {
        for layer in self.layers.iter().rev() {
            if let Some(pos) = layer.cursor_position(area, cx) {
                return pos;
            }
        }
        panic!("No layer returned a position!");
    }
}
