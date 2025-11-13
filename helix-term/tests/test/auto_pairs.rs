use helix_core::{auto_pairs::DEFAULT_PAIRS, hashmap};

use super::*;

const LINE_END: &str = helix_core::NATIVE_LINE_ENDING.as_str();

fn differing_pairs() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    DEFAULT_PAIRS.iter().filter(|(open, close)| open != close)
}

fn matching_pairs() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    DEFAULT_PAIRS.iter().filter(|(open, close)| open == close)
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("#[{}|]#", LINE_END),
            format!("i{}", pair.0),
            format!("{}#[|{}]#{}", pair.0, pair.1, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_multi_character_pairs() -> anyhow::Result<()> {
    let pairs = hashmap!("\\(".into() => "\\)".into(), "<|".into() => "|>".into(), "```".into() => "```".into());

    let config = Config {
        editor: helix_view::editor::Config {
            auto_pairs: AutoPairConfig::Pairs(pairs.clone()),
            ..Default::default()
        },
        ..Default::default()
    };

    for (open, close) in pairs.iter() {
        let mut chars = open.chars();
        let open_last = chars.next_back().unwrap();
        let open_but_last: String = chars.collect();

        let mut chars = close.chars();
        let close_head = chars.next().unwrap();
        let close_tail: String = chars.collect();
        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("{}#[{}|]#", open_but_last, LINE_END),
                format!("i{}", open_last),
                format!("{}#[|{}]#{}{}", open, close_head, close_tail, LINE_END),
                LineFeedHandling::AsIs,
            ),
        )
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_configured_multi_byte_chars() -> anyhow::Result<()> {
    // NOTE: these are multi-byte Unicode characters
    let pairs =
        hashmap!("„".into() => "“".into(), "‚".into() => "‘".into(), "「".into() => "」".into());

    let config = Config {
        editor: helix_view::editor::Config {
            auto_pairs: AutoPairConfig::Pairs(pairs.clone()),
            ..Default::default()
        },
        ..Default::default()
    };

    for (open, close) in pairs.iter() {
        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("#[{}|]#", LINE_END),
                format!("i{}", open),
                format!("{}#[|{}]#{}", open, close, LINE_END),
                LineFeedHandling::AsIs,
            ),
        )
        .await?;

        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("{}#[{}|]#{}", open, close, LINE_END),
                format!("i{}", close),
                format!("{}{}#[|{}]#", open, close, LINE_END),
                LineFeedHandling::AsIs,
            ),
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_after_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("foo#[{}|]#", LINE_END),
            format!("i{}", pair.0),
            format!("foo{}#[|{}]#{}", pair.0, pair.1, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    for pair in matching_pairs() {
        test((
            format!("foo#[{}|]#", LINE_END),
            format!("i{}", pair.0),
            format!("foo{}#[|{}]#", pair.0, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("#[f|]#oo{}", LINE_END),
            format!("i{}", pair.0),
            format!("{}#[|f]#oo{}", pair.0, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_word_selection() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("#[foo|]#{}", LINE_END),
            format!("i{}", pair.0),
            format!("{}#[|foo]#{}", pair.0, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_word_selection_trailing_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("foo#[ wor|]#{}", LINE_END),
            format!("i{}", pair.0),
            format!("foo{}#[|{} wor]#{}", pair.0, pair.1, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_closer_selection_trailing_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("foo{}#[|{} wor]#{}", pair.0, pair.1, LINE_END),
            format!("i{}", pair.1),
            format!("foo{}{}#[| wor]#{}", pair.0, pair.1, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_eol() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("{0}#[{0}|]#", LINE_END),
            format!("i{}", pair.0),
            format!(
                "{eol}{open}#[|{close}]#{eol}",
                eol = LINE_END,
                open = pair.0,
                close = pair.1
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_auto_pairs_disabled() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_with_config(
            AppBuilder::new().with_config(Config {
                editor: helix_view::editor::Config {
                    auto_pairs: AutoPairConfig::Enable(false),
                    ..Default::default()
                },
                ..Default::default()
            }),
            (
                format!("#[{}|]#", LINE_END),
                format!("i{}", pair.0),
                format!("{}#[|{}]#", pair.0, LINE_END),
                LineFeedHandling::AsIs,
            ),
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_multi_range() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("#[{eol}|]##({eol}|)##({eol}|)#", eol = LINE_END),
            format!("i{}", pair.0),
            format!(
                "{open}#[|{close}]#{eol}{open}#(|{close})#{eol}{open}#(|{close})#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_multi_code_point_graphemes() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("hello #[👨‍👩‍👧‍👦|]# goodbye{}", LINE_END),
            format!("i{}", pair.1),
            format!("hello {}#[|👨‍👩‍👧‍👦]# goodbye{}", pair.1, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_at_end_of_document() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test(TestCase {
            in_text: String::from(LINE_END),
            in_selection: Selection::single(LINE_END.len(), LINE_END.len()),
            in_keys: format!("i{}", pair.0),
            out_text: format!("{}{}{}", LINE_END, pair.0, pair.1),
            out_selection: Selection::single(LINE_END.len() + 1, LINE_END.len() + 2),
            line_feed_handling: LineFeedHandling::AsIs,
        })
        .await?;

        test(TestCase {
            in_text: format!("foo{}", LINE_END),
            in_selection: Selection::single(3 + LINE_END.len(), 3 + LINE_END.len()),
            in_keys: format!("i{}", pair.0),
            out_text: format!("foo{}{}{}", LINE_END, pair.0, pair.1),
            out_selection: Selection::single(LINE_END.len() + 4, LINE_END.len() + 5),
            line_feed_handling: LineFeedHandling::AsIs,
        })
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_close_inside_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "{open}#[{close}|]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            format!("i{}", pair.1),
            format!(
                "{open}{close}#[|{eol}]#",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_close_inside_pair_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "{open}#[{close}|]#{eol}{open}#({close}|)#{eol}{open}#({close}|)#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            format!("i{}", pair.1),
            format!(
                "{open}{close}#[|{eol}]#{open}{close}#(|{eol})#{open}{close}#(|{eol})#",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_nested_open_inside_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!(
                "{open}#[{close}|]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            format!("i{}", pair.0),
            format!(
                "{open}{open}#[|{close}]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_nested_open_inside_pair_multi() -> anyhow::Result<()> {
    for outer_pair in DEFAULT_PAIRS {
        for inner_pair in DEFAULT_PAIRS {
            if inner_pair.0 == outer_pair.0 {
                continue;
            }

            test((
                format!(
                    "{outer_open}#[{outer_close}|]#{eol}{outer_open}#({outer_close}|)#{eol}{outer_open}#({outer_close}|)#{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    eol = LINE_END
                ),
                format!("i{}", inner_pair.0),
                format!(
                    "{outer_open}{inner_open}#[|{inner_close}]#{outer_close}{eol}{outer_open}{inner_open}#(|{inner_close})#{outer_close}{eol}{outer_open}{inner_open}#(|{inner_close})#{outer_close}{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    inner_open = inner_pair.0,
                    inner_close = inner_pair.1,
                    eol = LINE_END
                ),
                LineFeedHandling::AsIs,
            ))
            .await?;
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("#[{}|]#", LINE_END),
            format!("a{}", pair.0),
            format!(
                "#[{eol}{open}{close}|]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_multi_character_pairs() -> anyhow::Result<()> {
    let pairs = hashmap!("\\(".into() => "\\)".into(), "<|".into() => "|>".into(), "```".into() => "```".into());

    let config = Config {
        editor: helix_view::editor::Config {
            auto_pairs: AutoPairConfig::Pairs(pairs.clone()),
            ..Default::default()
        },
        ..Default::default()
    };

    for (open, close) in pairs.iter() {
        let mut chars = open.chars();
        let open_last = chars.next_back().unwrap();
        let open_but_last: String = chars.collect();

        let mut chars = close.chars();
        let close_head = chars.next().unwrap();
        let close_tail: String = chars.collect();
        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("#[{}{}|]#", LINE_END, open_but_last),
                format!("a{}", open_last),
                format!(
                    "#[{eol}{open}{close_head}|]#{close_tail}{eol}",
                    eol = LINE_END
                ),
                LineFeedHandling::AsIs,
            ),
        )
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_multi_range() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("#[ |]#{eol}#( |)#{eol}#( |)#{eol}", eol = LINE_END),
            format!("a{}", pair.0),
            format!(
                "#[ {open}{close}|]#{eol}#( {open}{close}|)#{eol}#( {open}{close}|)#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_close_inside_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "#[{open}|]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            format!("a{}", pair.1),
            format!(
                "#[{open}{close}{eol}|]#",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_close_inside_pair_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "#[{open}|]#{close}{eol}#({open}|)#{close}{eol}#({open}|)#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            format!("a{}", pair.1),
            format!(
                "#[{open}{close}{eol}|]##({open}{close}{eol}|)##({open}{close}{eol}|)#",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_end_of_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("fo#[o|]#{}", LINE_END),
            format!("a{}", pair.0),
            format!(
                "fo#[o{open}{close}|]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_middle_of_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("#[wo|]#rd{}", LINE_END),
            format!("a{}", pair.1),
            format!("#[wo{}r|]#d{}", pair.1, LINE_END),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_end_of_word_multi() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!("fo#[o|]#{eol}fo#(o|)#{eol}fo#(o|)#{eol}", eol = LINE_END),
            format!("a{}", pair.0),
            format!(
                "fo#[o{open}{close}|]#{eol}fo#(o{open}{close}|)#{eol}fo#(o{open}{close}|)#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_inside_nested_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!(
                "f#[oo{open}|]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            format!("a{}", pair.0),
            format!(
                "f#[oo{open}{open}{close}|]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            LineFeedHandling::AsIs,
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_inside_nested_pair_multi() -> anyhow::Result<()> {
    for outer_pair in DEFAULT_PAIRS {
        for inner_pair in DEFAULT_PAIRS {
            if inner_pair.0 == outer_pair.0 {
                continue;
            }

            test((
                format!(
                    "f#[oo{outer_open}|]#{outer_close}{eol}f#(oo{outer_open}|)#{outer_close}{eol}f#(oo{outer_open}|)#{outer_close}{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    eol = LINE_END
                ),
                format!("a{}", inner_pair.0),
                format!(
                    "f#[oo{outer_open}{inner_open}{inner_close}|]#{outer_close}{eol}f#(oo{outer_open}{inner_open}{inner_close}|)#{outer_close}{eol}f#(oo{outer_open}{inner_open}{inner_close}|)#{outer_close}{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    inner_open = inner_pair.0,
                    inner_close = inner_pair.1,
                    eol = LINE_END
                ),
                LineFeedHandling::AsIs,
            ))
            .await?;
        }
    }

    Ok(())
}
