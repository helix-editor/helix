use crate::{buffer::Buffer, widgets::Widget};

use steel::{gc::unsafe_erased_pointers::CustomReference, rvals::Custom};

impl CustomReference for Buffer {}
