//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use std::path::{Path, PathBuf};

pub fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn book_gen() -> PathBuf {
    project_root().join("book/src/generated/")
}

pub fn ts_queries() -> PathBuf {
    project_root().join("runtime/queries")
}

pub fn lang_config() -> PathBuf {
    project_root().join("languages.toml")
}
