use steel::{gc::unsafe_erased_pointers::CustomReference, rvals::Custom};

use crate::{graphics::Rect, input::Event};

impl CustomReference for Event {}
impl Custom for Rect {}
impl Custom for crate::graphics::CursorKind {}
