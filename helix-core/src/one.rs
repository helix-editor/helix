//! Make all your struct fields public and use thsese inside of them! (parse don't validate, invalid states unrepresentable, tightness driven dev, yada yada)
//! 
//! Various structs that are nonempty.
mod str1;
mod vec1;

pub use str1::{Str1, str1};
pub use vec1::Vec1;
