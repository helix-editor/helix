// IDEA: render to a cache buffer, then if not changed, copy the buf into the parent
type Surface = ();
pub trait Component {
    /// Process input events, return true if handled.
    fn process_event(&mut self, event: crossterm::event::Event, args: ()) -> bool;
    /// Should redraw? Useful for saving redraw cycles if we know component didn't change.
    fn should_update(&self) -> bool {
        true
    }

    fn render(&mut self, surface: &mut Surface, args: ());
}

// HStack / VStack
// focus by component id: each View/Editor gets it's own incremental id at create
// Component: View(Arc<State>) -> multiple views can point to same state
// id 0 = prompt?
// when entering to prompt, it needs to direct Commands to last focus window
// -> prompt.trigger(focus_id), on_leave -> focus(focus_id)
// popups on another layer
