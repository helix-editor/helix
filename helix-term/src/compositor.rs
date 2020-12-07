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

use crate::application::Renderer;
use crossterm::event::Event;
use smol::Executor;
use tui::buffer::Buffer as Surface;

pub(crate) type Callback = Box<dyn Fn(&mut Compositor)>;

// Cursive-inspired
pub(crate) enum EventResult {
    Ignored,
    Consumed(Option<Callback>),
}

pub(crate) trait Component {
    /// Process input events, return true if handled.
    fn handle_event(&mut self, event: Event, executor: &Executor) -> EventResult;
    // , args: ()

    /// Should redraw? Useful for saving redraw cycles if we know component didn't change.
    fn should_update(&self) -> bool {
        true
    }

    fn render(&mut self, renderer: &mut Renderer);
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

pub(crate) struct Compositor {
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

    pub fn handle_event(&mut self, event: Event, executor: &Executor) -> () {
        // TODO: custom focus
        if let Some(layer) = self.layers.last_mut() {
            layer.handle_event(event, executor);
            // return should_update
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        for layer in &mut self.layers {
            layer.render(renderer)
        }
    }
}
