use bitflags::bitflags;

use crate::symbols::line;

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
    pub struct Borders: u8 {
        /// Show the top border
        const TOP = 0b0000_0001;
        /// Show the right border
        const RIGHT = 0b0000_0010;
        /// Show the bottom border
        const BOTTOM = 0b000_0100;
        /// Show the left border
        const LEFT = 0b0000_1000;
        /// Show all borders
        const ALL = Self::TOP.bits() | Self::RIGHT.bits() | Self::BOTTOM.bits() | Self::LEFT.bits();
    }
}

/// Border render type. Defaults to [`BorderType::Plain`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BorderType {
    #[default]
    Plain,
    Rounded,
    Double,
    Thick,
}

impl BorderType {
    pub fn line_symbols(border_type: Self) -> line::Set {
        match border_type {
            Self::Plain => line::NORMAL,
            Self::Rounded => line::ROUNDED,
            Self::Double => line::DOUBLE,
            Self::Thick => line::THICK,
        }
    }
}
