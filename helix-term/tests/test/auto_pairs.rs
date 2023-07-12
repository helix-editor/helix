use helix_core::{auto_pairs::DEFAULT_PAIRS, hashmap, syntax::AutoPairConfig};
use helix_term::config::Config;

use crate::{test::helpers::AppBuilder, test_case};

fn differing_pairs() -> impl Iterator<Item = &'static (char, char)> {
    DEFAULT_PAIRS.iter().filter(|(open, close)| open != close)
}

fn matching_pairs() -> impl Iterator<Item = &'static (char, char)> {
    DEFAULT_PAIRS.iter().filter(|(open, close)| open == close)
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(("#[\n|]#"), ("i{}", pair.0), ("{}#[|{}]#", pair.0, pair.1)).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_configured_multi_byte_chars() -> anyhow::Result<()> {
    // NOTE: these are multi-byte Unicode characters
    let pairs = hashmap!('„' => '“', '‚' => '‘', '「' => '」');

    let config = Config {
        editor: helix_view::editor::Config {
            auto_pairs: AutoPairConfig::Pairs(pairs.clone()),
            ..Default::default()
        },
        ..Default::default()
    };

    for (open, close) in pairs.iter() {
        test_case!(
            AppBuilder::default().with_config(config.clone()),
            ("#[\n|]#"),
            ("i{}", open),
            ("{}#[|{}]#", open, close)
        )
        .await?;

        test_case!(
            AppBuilder::default().with_config(config.clone()),
            ("{}#[{}|]#", open, close),
            ("i{}", close),
            ("{}{}#[|\n]#", open, close)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_after_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("foo#[\n|]#"),
            ("i{}", pair.0),
            ("foo{}#[|{}]#", pair.0, pair.1)
        )
        .await?;
    }

    for pair in matching_pairs() {
        test_case!(("foo#[\n|]#"), ("i{}", pair.0), ("foo{}#[|\n]#", pair.0)).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(("#[f|]#oo"), ("i{}", pair.0), ("{}#[|f]#oo", pair.0)).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_word_selection() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(("#[foo|]#"), ("i{}", pair.0), ("{}#[|foo]#", pair.0)).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_word_selection_trailing_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("foo#[ wor|]#"),
            ("i{}", pair.0),
            ("foo{}#[|{} wor]#", pair.0, pair.1)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_closer_selection_trailing_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("foo{}#[|{} wor]#", pair.0, pair.1),
            ("i{}", pair.1),
            ("foo{}{}#[| wor]#", pair.0, pair.1)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_eol() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            ("\n#[\n|]#"),
            ("i{}", pair.0),
            ("\n{open}#[|{close}]#", open = pair.0, close = pair.1,)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_auto_pairs_disabled() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            AppBuilder::default().with_config(Config {
                editor: helix_view::editor::Config {
                    auto_pairs: AutoPairConfig::Enable(false),
                    ..Default::default()
                },
                ..Default::default()
            }),
            ("#[\n|]#"),
            ("i{}", pair.0),
            ("{}#[|\n]#", pair.0)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_multi_range() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            ("#[\n|]##(\n|)##(\n|)#"),
            ("i{}", pair.0),
            (
                "{open}#[|{close}]#\n{open}#(|{close})#\n{open}#(|{close})#",
                open = pair.0,
                close = pair.1,
            )
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_before_multi_code_point_graphemes() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("hello #[👨‍👩‍👧‍👦|]# goodbye"),
            ("i{}", pair.1),
            ("hello {}#[|👨‍👩‍👧‍👦]# goodbye", pair.1)
        )
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_at_end_of_document() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(("#[|]#"), ("i{}", pair.0), ("{}#[|{}]#", pair.0, pair.1)).await?;

        // HELP: is this intentional?
        const QUOTE_CHARS: [char; 3] = ['\'', '"', '`'];
        if QUOTE_CHARS.contains(&pair.0) {
            continue;
        }

        test_case!(
            ("foo#[|]#"),
            ("i{}", pair.0),
            ("foo{}#[|{}]#", pair.0, pair.1)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_close_inside_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            ("{open}#[{close}|]#", open = pair.0, close = pair.1,),
            ("i{}", pair.1),
            ("{open}{close}#[|\n]#", open = pair.0, close = pair.1,)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_close_inside_pair_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            (
                "{open}#[{close}|]#\n{open}#({close}|)#\n{open}#({close}|)#",
                open = pair.0,
                close = pair.1,
            ),
            ("i{}", pair.1),
            (
                "{open}{close}#[|\n]#{open}{close}#(|\n)#{open}{close}#(|\n)#",
                open = pair.0,
                close = pair.1,
            )
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_nested_open_inside_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("{open}#[{close}|]#", open = pair.0, close = pair.1,),
            ("i{}", pair.0),
            (
                "{open}{open}#[|{close}]#{close}",
                open = pair.0,
                close = pair.1,
            )
        )
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

            test_case!(
                (
                    "{outer_open}#[{outer_close}|]#\n{outer_open}#({outer_close}|)#\n{outer_open}#({outer_close}|)#",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                ),
                ("i{}", inner_pair.0),
                (
                    "{outer_open}{inner_open}#[|{inner_close}]#{outer_close}\n{outer_open}{inner_open}#(|{inner_close})#{outer_close}\n{outer_open}{inner_open}#(|{inner_close})#{outer_close}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    inner_open = inner_pair.0,
                    inner_close = inner_pair.1,
                )
            )
            .await?;
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            ("#[\n|]#"),
            ("a{}", pair.0),
            ("#[\n{open}{close}|]#", open = pair.0, close = pair.1,)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_multi_range() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            ("#[ |]#\n#( |)#\n#( |)#"),
            ("a{}", pair.0),
            (
                "#[ {open}{close}|]#\n#( {open}{close}|)#\n#( {open}{close}|)#",
                open = pair.0,
                close = pair.1,
            )
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_close_inside_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            ("#[{open}|]#{close}", open = pair.0, close = pair.1,),
            ("a{}", pair.1),
            ("#[{open}{close}\n|]#", open = pair.0, close = pair.1,)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_close_inside_pair_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test_case!(
            (
                "#[{open}|]#{close}\n#({open}|)#{close}\n#({open}|)#{close}",
                open = pair.0,
                close = pair.1,
            ),
            ("a{}", pair.1),
            (
                "#[{open}{close}\n|]##({open}{close}\n|)##({open}{close}\n|)#",
                open = pair.0,
                close = pair.1,
            )
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_end_of_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("fo#[o|]#"),
            ("a{}", pair.0),
            ("fo#[o{open}{close}|]#", open = pair.0, close = pair.1,)
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_middle_of_word() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(("#[wo|]#rd"), ("a{}", pair.1), ("#[wo{}r|]#d", pair.1)).await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_end_of_word_multi() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("fo#[o|]#\nfo#(o|)#\nfo#(o|)#"),
            ("a{}", pair.0),
            (
                "fo#[o{open}{close}|]#\nfo#(o{open}{close}|)#\nfo#(o{open}{close}|)#",
                open = pair.0,
                close = pair.1,
            )
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_inside_nested_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test_case!(
            ("f#[oo{open}|]#{close}", open = pair.0, close = pair.1,),
            ("a{}", pair.0),
            (
                "f#[oo{open}{open}{close}|]#{close}",
                open = pair.0,
                close = pair.1,
            )
        )
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

            test_case!(
                (
                    "f#[oo{outer_open}|]#{outer_close}\nf#(oo{outer_open}|)#{outer_close}\nf#(oo{outer_open}|)#{outer_close}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                ),
                ("a{}", inner_pair.0),
                (
                    "f#[oo{outer_open}{inner_open}{inner_close}|]#{outer_close}\nf#(oo{outer_open}{inner_open}{inner_close}|)#{outer_close}\nf#(oo{outer_open}{inner_open}{inner_close}|)#{outer_close}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    inner_open = inner_pair.0,
                    inner_close = inner_pair.1,
                )
            )
            .await?;
        }
    }

    Ok(())
}
