#[cfg(feature = "steel")]
mod steel_implementations {

    use crate::{
        buffer::Buffer,
        text::Text,
        widgets::{Block, List, Paragraph, Table},
    };

    use steel::{gc::unsafe_erased_pointers::CustomReference, rvals::Custom};

    impl CustomReference for Buffer {}
    impl Custom for Block<'static> {}
    impl Custom for List<'static> {}
    impl Custom for Paragraph<'static> {}
    impl Custom for Table<'static> {}
    impl Custom for Text<'static> {}

    steel::custom_reference!(Buffer);
}
