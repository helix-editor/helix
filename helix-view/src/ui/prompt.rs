// Prompt component depends on the compositor context so it cannot yet be fully moved here.

#[derive(Clone, Copy, PartialEq)]
pub enum PromptEvent {
    /// The prompt input has been updated.
    Update,
    /// Validate and finalize the change.
    Validate,
    /// Abort the change, reverting to the initial state.
    Abort,
}
