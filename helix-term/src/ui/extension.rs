#[cfg(feature = "steel")]
mod steel_implementations {

    use crate::{
        compositor::Component,
        ui::{overlay::Overlay, Popup, Prompt, Text},
    };

    impl steel::rvals::Custom for Text {}
    impl<T: steel::rvals::IntoSteelVal + Component> steel::rvals::Custom for Popup<T> {}

    // TODO: For this to be sound, all of the various functions
    // have to now be marked as send + sync + 'static. Annoying,
    // and something I'll look into with steel.
    unsafe impl<T> Send for Overlay<T> {}
    unsafe impl<T> Sync for Overlay<T> {}
    unsafe impl Send for Prompt {}
    unsafe impl Sync for Prompt {}
}
