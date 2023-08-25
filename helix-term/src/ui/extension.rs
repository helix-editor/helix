#[cfg(feature = "steel")]
mod steel_implementations {

    use crate::{
        compositor::Component,
        ui::{Popup, Text},
    };

    impl steel::rvals::Custom for Text {}
    impl<T: steel::rvals::IntoSteelVal + Component> steel::rvals::Custom for Popup<T> {}
}
