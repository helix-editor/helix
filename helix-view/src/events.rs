use helix_core::Rope;
use helix_event::events;

use crate::{Document, ViewId};

events! {
    DocumentDidChange<'a> { doc: &'a mut Document, view: ViewId, old_text: &'a Rope  }
    SelectionDidChange<'a> { doc: &'a mut Document, view: ViewId }
}
