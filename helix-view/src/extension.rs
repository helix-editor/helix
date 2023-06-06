use steel::{gc::unsafe_erased_pointers::CustomReference, rvals::Custom};

use crate::{graphics::Rect, input::Event, Document, DocumentId, ViewId};

// Reference types along with value types - This should allow for having users
impl CustomReference for Event {}
impl Custom for Rect {}
impl Custom for crate::graphics::CursorKind {}
impl Custom for DocumentId {}
impl Custom for ViewId {}
impl CustomReference for Document {}
