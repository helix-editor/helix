//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use smartstring::{LazyCompact, SmartString};
use textwrap::{Options, WordSplitter::NoHyphenation};

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(text: &str, text_width: usize) -> SmartString<LazyCompact> {
    let options = Options::new(text_width).word_splitter(NoHyphenation);
    textwrap::refill(text, options).into()
}
