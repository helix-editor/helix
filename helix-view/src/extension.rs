use crate::DocumentId;

pub fn document_id_to_usize(doc_id: &DocumentId) -> usize {
    doc_id.0.into()
}

#[cfg(feature = "steel")]
mod steel_implementations {

    use steel::{gc::unsafe_erased_pointers::CustomReference, rvals::Custom};

    use crate::{
        document::Mode, graphics::Rect, input::Event, Document, DocumentId, Editor, ViewId,
    };

    impl steel::gc::unsafe_erased_pointers::CustomReference for Editor {}
    steel::custom_reference!(Editor);

    impl steel::rvals::Custom for Mode {}
    impl steel::rvals::Custom for Event {}

    // Reference types along with value types - This should allow for having users
    impl CustomReference for Event {}
    impl Custom for Rect {}
    impl Custom for crate::graphics::CursorKind {}
    impl Custom for DocumentId {}
    impl Custom for ViewId {}
    impl CustomReference for Document {}
}
