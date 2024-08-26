//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.c", None),
        // switches to append mode?
        (
            "void foo() {#[|}]#",
            "i<ret><esc>",
            indoc! {"\
                void foo() {
                  #[|\n]#\
                }
            "},
        ),
    )
    .await?;

    Ok(())
}
