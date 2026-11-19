use helix_core::syntax::config::{HardWrap, ListMarkersConfig};

use super::*;

fn hard_wrap_config(text_width: usize, list_markers: bool) -> Config {
    let mut config = test_config();
    config.editor.text_width = text_width;
    config.editor.hard_wrap = HardWrap {
        enable: Some(true),
        list_markers: Some(ListMarkersConfig::Enable(list_markers)),
    };
    config
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_wraps_past_text_width() -> anyhow::Result<()> {
    // text-width 10: the second `c` is entered past the margin and breaks the
    // line at the last space before it
    test_with_config(
        AppBuilder::new().with_config(hard_wrap_config(10, true)),
        ("#[\n|]#", "iaaa bbbb ccc<esc>", "aaa bbbb\nccc#[|\n]#"),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_whitespace_never_wraps() -> anyhow::Result<()> {
    // spaces entered past the margin do not trigger a break (vim rule)
    test_with_config(
        AppBuilder::new().with_config(hard_wrap_config(10, true)),
        ("#[\n|]#", "iaaa bbbb <esc>", "aaa bbbb #[|\n]#"),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_never_splits_words() -> anyhow::Result<()> {
    // `longword` (8 wide) fills the width exactly at the end of the line and is
    // never split; the break happened before the word, keeping `aaaa` intact
    test_with_config(
        AppBuilder::new().with_config(hard_wrap_config(8, true)),
        ("#[\n|]#", "iaaaa longword<esc>", "aaaa\nlongword#[|\n]#"),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_list_marker_alignment() -> anyhow::Result<()> {
    // continuation lines of a markdown list item align under the item content
    test_with_config(
        AppBuilder::new().with_config(hard_wrap_config(10, true)),
        (
            "#[\n|]#",
            "i- aaa bbbb ccc<esc>",
            "- aaa bbbb\n  ccc#[|\n]#",
        ),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_list_marker_custom_regex() -> anyhow::Result<()> {
    // a custom `list-markers` regex replaces the built-in set: only ordered
    // markers (`1. `) align the continuation lines under the item content. The
    // list item is on the second line so the marker is not at char 0, which
    // exercises the byte-offset handling of the regex marker resolution.
    let mut config = test_config();
    config.editor.text_width = 10;
    config.editor.hard_wrap = HardWrap {
        enable: Some(true),
        list_markers: Some(ListMarkersConfig::Regex(r"^(\d+[.)]\s+)".to_string())),
    };
    test_with_config(
        AppBuilder::new().with_config(config),
        (
            "#[\n|]#",
            "ihello<ret>1. aaa bbbb ccc<esc>",
            "hello\n1. aaa\n   bbbb\n   ccc#[|\n]#",
        ),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_list_markers_disabled() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_config(hard_wrap_config(10, false)),
        ("#[\n|]#", "i- aaa bbbb ccc<esc>", "- aaa bbbb\nccc#[|\n]#"),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_disabled_by_default() -> anyhow::Result<()> {
    // hard wrap off: over-long lines are untouched
    test_with_config(
        AppBuilder::new(),
        ("#[\n|]#", "iaaa bbbb ccc<esc>", "aaa bbbb ccc#[|\n]#"),
    )
    .await
}
