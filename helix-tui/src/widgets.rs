//! `widgets` is a collection of types that implement [`Widget`].
//!
//! All widgets are implemented using the builder pattern and are consumable objects. They are not
//! meant to be stored but used as *commands* to draw common figures in the UI.
//!
//! The available widgets are:
//! - [`Block`]
// //! - [`List`]
// //! - [`Table`]
//! - [`Paragraph`]

mod borders;
pub use borders::{BorderType, Borders};

mod widget;
pub use widget::Widget;

mod block;
pub use block::Block;

// mod list;
// pub use self::list::{List, ListItem, ListState};

mod paragraph;
pub use paragraph::{Paragraph, Wrap};

mod reflow;

mod table;
pub use table::{Cell, Row, Table, TableState};
