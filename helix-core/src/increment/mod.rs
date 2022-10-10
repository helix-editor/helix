pub mod date_time;
pub mod integer;

use crate::{Range, Tendril};

pub trait Increment {
    fn increment(&self, amount: i64) -> (Range, Tendril);
}
