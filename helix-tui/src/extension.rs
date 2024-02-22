#[cfg(feature = "steel")]
mod steel_implementations {

    use crate::{buffer::Buffer, widgets::Block};

    use steel::{gc::unsafe_erased_pointers::CustomReference, rvals::Custom};

    impl CustomReference for Buffer {}
    impl Custom for Block<'static> {}

    steel::custom_reference!(Buffer);
}
