//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent() -> anyhow::Result<()> {
    let app = || AppBuilder::new().with_file("foo.go", None);

    let enter_tests = [
        (
            indoc! {r##"
                type Test struct {#[}|]#
            "##},
            "i<ret>",
            indoc! {"\
                type Test struct {
                \t#[|\n]#
                }
            "},
        ),
        (
            indoc! {"\
                func main() {
                \tswitch nil {#[}|]#
                }
             "},
            "i<ret>",
            indoc! {"\
                func main() {
                \tswitch nil {
                \t\t#[|\n]#
                \t}
                }
            "},
        ),
    ];

    for test in enter_tests {
        test_with_config(app(), test).await?;
    }

    Ok(())
}
