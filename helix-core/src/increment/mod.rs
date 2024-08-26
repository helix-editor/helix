//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


mod date_time;
mod integer;

pub fn integer(selected_text: &str, amount: i64) -> Option<String> {
    integer::increment(selected_text, amount)
}

pub fn date_time(selected_text: &str, amount: i64) -> Option<String> {
    date_time::increment(selected_text, amount)
}
