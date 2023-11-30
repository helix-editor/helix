use crate::{DocumentId, ViewId};

#[derive(Debug, Clone, Copy)]
pub struct CompletionTrigger {
    /// The char position of the primary cursor when the
    /// completion was triggered
    pub trigger_pos: usize,
    pub doc: DocumentId,
    pub view: ViewId,
    /// Whether the cause of the trigger was an automatic completion (any word
    /// char for words longer than minimum word length).
    /// This is false for trigger chars send by the LS
    pub auto: bool,
}

pub enum CompletionEvent {
    /// Auto completion was triggered by typing a word char
    /// or a completion trigger
    Trigger(CompletionTrigger),
    /// A completion was manually requested (c-x)
    Manual,
    /// Some text was deleted and the cursor is now at `pos`
    DeleteText { pos: usize },
    /// Invalidate the current auto completion trigger
    Cancel,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SignatureHelpInvoked {
    Automatic,
    Manual,
}

pub enum SignatureHelpEvent {
    Invoked,
    Trigger,
    ReTrigger,
    Cancel,
    RequestComplete { open: bool },
}
