//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use anyhow::Result;
use helix_loader::grammar::fetch_grammars;

// This binary is used in the Release CI as an optimization to cut down on
// compilation time. This is not meant to be run manually.

fn main() -> Result<()> {
    fetch_grammars()
}
