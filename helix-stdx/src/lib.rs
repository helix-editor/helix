//! Extensions to the standard library. A collection of helper functions
//! used throughout helix.

pub mod env;
pub mod faccess;
pub mod path;
pub mod range;
pub mod rope;
pub mod uri;

pub use range::Range;
pub use uri::Url;
