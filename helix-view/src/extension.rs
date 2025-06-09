use crate::DocumentId;

pub fn document_id_to_usize(doc_id: &DocumentId) -> usize {
    doc_id.0.into()
}

#[cfg(feature = "steel")]
mod steel_implementations {

    use steel::{
        gc::unsafe_erased_pointers::CustomReference,
        rvals::{as_underlying_type, Custom},
    };

    use crate::{
        document::Mode,
        editor::{
            Action, AutoSave, BufferLine, CursorShapeConfig, FilePickerConfig, GutterConfig,
            IndentGuidesConfig, LineEndingConfig, LineNumber, LspConfig, SearchConfig,
            SmartTabConfig, StatusLineConfig, TerminalConfig, WhitespaceConfig,
        },
        graphics::{Color, Rect, Style, UnderlineStyle},
        input::Event,
        Document, DocumentId, Editor, ViewId,
    };

    impl steel::gc::unsafe_erased_pointers::CustomReference for Editor {}
    steel::custom_reference!(Editor);

    impl steel::rvals::Custom for Mode {
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<Self>(other) {
                self == other
            } else {
                false
            }
        }
    }
    impl steel::rvals::Custom for Event {}
    impl Custom for Style {
        fn fmt(&self) -> Option<std::result::Result<String, std::fmt::Error>> {
            Some(Ok(format!("{:?}", self)))
        }
    }
    impl Custom for Color {
        fn fmt(&self) -> Option<std::result::Result<String, std::fmt::Error>> {
            Some(Ok(format!("{:?}", self)))
        }
    }
    impl Custom for UnderlineStyle {}

    impl CustomReference for Event {}
    impl Custom for Rect {
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<Rect>(other) {
                self == other
            } else {
                false
            }
        }
    }
    impl Custom for crate::graphics::CursorKind {
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<Self>(other) {
                self == other
            } else {
                false
            }
        }
    }
    impl Custom for DocumentId {
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<DocumentId>(other) {
                self == other
            } else {
                false
            }
        }
    }
    impl Custom for ViewId {
        fn equality_hint(&self, other: &dyn steel::rvals::CustomType) -> bool {
            if let Some(other) = as_underlying_type::<ViewId>(other) {
                self == other
            } else {
                false
            }
        }
    }
    impl CustomReference for Document {}

    impl Custom for Action {}

    impl Custom for FilePickerConfig {}
    impl Custom for StatusLineConfig {}
    impl Custom for SearchConfig {}
    impl Custom for TerminalConfig {}
    impl Custom for WhitespaceConfig {}
    impl Custom for CursorShapeConfig {}
    impl Custom for BufferLine {}
    impl Custom for LineNumber {}
    impl Custom for GutterConfig {}
    impl Custom for LspConfig {}
    impl Custom for IndentGuidesConfig {}
    impl Custom for LineEndingConfig {}
    impl Custom for SmartTabConfig {}
    impl Custom for AutoSave {}
}
