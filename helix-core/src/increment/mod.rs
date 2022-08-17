pub mod date_time;
pub mod number;
pub mod ordered_list;

use crate::{Range, Tendril};

pub trait Increment {
    fn increment(&self, amount: i64) -> (Range, Tendril);
}
