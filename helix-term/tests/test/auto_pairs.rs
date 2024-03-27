use helix_core::{auto_pairs::DEFAULT_PAIRS, hashmap};

use super::*;

const LINE_END: &str = helix_core::NATIVE_LINE_ENDING.as_str();

fn differing_pairs() -> impl Iterator<Item = &'static (char, char)> {
    DEFAULT_PAIRS.iter().filter(|(open, close)| open != close)
}

fn matching_pairs() -> impl Iterator<Item = &'static (char, char)> {
    DEFAULT_PAIRS.iter().filter(|(open, close)| open == close)
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line("#[\n|]#"),
            format!("i{}", pair.0),
            helpers::platform_line(&format!("{}#[|{}]#", pair.0, pair.1)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_whitespace() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!("{}#[|{}]#", pair.0, pair.1)),
            "i ",
            helpers::platform_line(&format!("{} #[| ]#{}", pair.0, pair.1)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_whitespace_multi() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    {open}#[|{close}]#
                    {open}#(|{open})#{close}{close}
                    {open}{open}#(|{close}{close})#
                    foo#(|\n)#
                "},
                open = pair.0,
                close = pair.1,
            )),
            "i ",
            helpers::platform_line(&format!(
                indoc! {"\
                    {open} #[| ]#{close}
                    {open} #(|{open})#{close}{close}
                    {open}{open} #(| {close}{close})#
                    foo #(|\n)#
                "},
                open = pair.0,
                close = pair.1,
            )),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_whitespace_multi() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    #[|{open}]#{close}
                    #(|{open})#{open}{close}{close}
                    #(|{open}{open})#{close}{close}
                    #(|foo)#
                "},
                open = pair.0,
                close = pair.1,
            )),
            "a ",
            helpers::platform_line(&format!(
                indoc! {"\
                    #[{open}  |]#{close}
                    #({open} {open}|)#{close}{close}
                    #({open}{open}  |)#{close}{close}
                    #(foo \n|)#
                "},
                open = pair.0,
                close = pair.1,
            )),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_whitespace_no_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        // sanity check - do not insert extra whitespace unless immediately
        // surrounded by a pair
        test((
            helpers::platform_line(&format!("{} #[|{}]#", pair.0, pair.1)),
            "i ",
            helpers::platform_line(&format!("{}  #[|{}]#", pair.0, pair.1)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_whitespace_no_matching_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        // sanity check - verify whitespace does not insert unless both pairs
        // are matches, i.e. no two different openers
        test((
            helpers::platform_line(&format!("{}#[|{}]#", pair.0, pair.0)),
            "i ",
            helpers::platform_line(&format!("{} #[|{}]#", pair.0, pair.0)),
        ))
        .await?;
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
        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("#[{}|]#", LINE_END),
                format!("i{}", open),
                format!("{}#[|{}]#{}", open, close, LINE_END),
            ),
        )
        .await?;

        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("{}#[{}|]#{}", open, close, LINE_END),
                format!("i{}", close),
                format!("{}{}#[|{}]#", open, close, LINE_END),
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
        ))
        .await?;
    }

    for pair in matching_pairs() {
        test((
            format!("foo#[{}|]#", LINE_END),
            format!("i{}", pair.0),
            format!("foo{}#[|{}]#", pair.0, LINE_END),
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
        })
        .await?;

        test(TestCase {
            in_text: format!("foo{}", LINE_END),
            in_selection: Selection::single(3 + LINE_END.len(), 3 + LINE_END.len()),
            in_keys: format!("i{}", pair.0),
            out_text: format!("foo{}{}{}", LINE_END, pair.0, pair.1),
            out_selection: Selection::single(LINE_END.len() + 4, LINE_END.len() + 5),
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
        ))
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
            ))
            .await?;
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("{}#[|{}]#{}", pair.0, pair.1, LINE_END),
            "i<backspace>",
            format!("#[|{}]#", LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    {open}#[|{close}]#
                    {open}#(|{close})#
                    {open}#(|{close})#
                "},
                open = pair.0,
                close = pair.1,
            )),
            "i<backspace>",
            helpers::platform_line(indoc! {"\
                #[|\n]#
                #(|\n)#
                #(|\n)#
            "}),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_whitespace() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!("{} #[| ]#{}", pair.0, pair.1)),
            "i<backspace>",
            helpers::platform_line(&format!("{}#[|{}]#", pair.0, pair.1)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_whitespace_after_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!("foo{} #[| ]#{}", pair.0, pair.1)),
            "i<backspace>",
            helpers::platform_line(&format!("foo{}#[|{}]#", pair.0, pair.1)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_whitespace_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    {open} #[| ]#{close}
                    {open} #(|{open})#{close}{close}
                    {open}{open} #(| {close}{close})#
                    foo #(|\n)#
                "},
                open = pair.0,
                close = pair.1,
            )),
            "i<backspace>",
            helpers::platform_line(&format!(
                indoc! {"\
                    {open}#[|{close}]#
                    {open}#(|{open})#{close}{close}
                    {open}{open}#(|{close}{close})#
                    foo#(|\n)#
                "},
                open = pair.0,
                close = pair.1,
            )),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_whitespace_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    #[{open} |]# {close}
                    #({open} |)#{open}{close}{close}
                    #({open}{open} |)# {close}{close}
                    #(foo |)#
                "},
                open = pair.0,
                close = pair.1,
            )),
            "a<backspace>",
            helpers::platform_line(&format!(
                indoc! {"\
                    #[{open}{close}|]#
                    #({open}{open}|)#{close}{close}
                    #({open}{open}{close}|)#{close}
                    #(foo\n|)#
                "},
                open = pair.0,
                close = pair.1,
            )),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_whitespace_no_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!("{}  #[|{}]#", pair.0, pair.1)),
            "i<backspace>",
            helpers::platform_line(&format!("{} #[|{}]#", pair.0, pair.1)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_whitespace_no_matching_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line(&format!("{} #[|{}]#", pair.0, pair.0)),
            "i<backspace>",
            helpers::platform_line(&format!("{}#[|{}]#", pair.0, pair.0)),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_configured_multi_byte_chars() -> anyhow::Result<()> {
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
        test_with_config(
            AppBuilder::new().with_config(config.clone()),
            (
                format!("{}#[|{}]#{}", open, close, LINE_END),
                "i<backspace>",
                format!("#[|{}]#", LINE_END),
            ),
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_after_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!("foo{}#[|{}]#", pair.0, pair.1)),
            "i<backspace>",
            helpers::platform_line("foo#[|\n]#"),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_then_delete() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line("#[\n|]#\n"),
            format!("ofoo{}<backspace>", pair.0),
            helpers::platform_line("\nfoo#[\n|]#\n"),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_then_delete_whitespace() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line("foo#[\n|]#"),
            format!("i{}<space><backspace><backspace>", pair.0),
            helpers::platform_line("foo#[|\n]#"),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_then_delete_multi() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line(indoc! {"\
                through a day#[\n|]#
                in and out of weeks#(\n|)#
                over a year#(\n|)#
            "}),
            format!("i{}<space><backspace><backspace>", pair.0),
            helpers::platform_line(indoc! {"\
                through a day#[|\n]#
                in and out of weeks#(|\n)#
                over a year#(|\n)#
            "}),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_then_delete() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line("fo#[o|]#"),
            format!("a{}<space><backspace><backspace>", pair.0),
            helpers::platform_line("fo#[o\n|]#"),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn append_then_delete_multi() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            helpers::platform_line(indoc! {"\
                #[through a day|]#
                #(in and out of weeks|)#
                #(over a year|)#
            "}),
            format!("a{}<space><backspace><backspace>", pair.0),
            helpers::platform_line(indoc! {"\
                #[through a day\n|]#
                #(in and out of weeks\n|)#
                #(over a year\n|)#
            "}),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_before_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        // sanity check unclosed pair delete
        test((
            format!("{}#[|f]#oo{}", pair.0, LINE_END),
            "i<backspace>",
            format!("#[|f]#oo{}", LINE_END),
        ))
        .await?;

        // deleting the closing pair should NOT delete the whole pair
        test((
            format!("{}{}#[|f]#oo{}", pair.0, pair.1, LINE_END),
            "i<backspace>",
            format!("{}#[|f]#oo{}", pair.0, LINE_END),
        ))
        .await?;

        // deleting whole pair before word
        test((
            format!("{}#[|{}]#foo{}", pair.0, pair.1, LINE_END),
            "i<backspace>",
            format!("#[|f]#oo{}", LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_before_word_selection() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        // sanity check unclosed pair delete
        test((
            format!("{}#[|foo]#{}", pair.0, LINE_END),
            "i<backspace>",
            format!("#[|foo]#{}", LINE_END),
        ))
        .await?;

        // deleting the closing pair should NOT delete the whole pair
        test((
            format!("{}{}#[|foo]#{}", pair.0, pair.1, LINE_END),
            "i<backspace>",
            format!("{}#[|foo]#{}", pair.0, LINE_END),
        ))
        .await?;

        // deleting whole pair before word
        test((
            format!("{}#[|{}foo]#{}", pair.0, pair.1, LINE_END),
            "i<backspace>",
            format!("#[|foo]#{}", LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_before_word_selection_trailing_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("foo{}#[|{} wor]#{}", pair.0, pair.1, LINE_END),
            "i<backspace>",
            format!("foo#[| wor]#{}", LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_before_eol() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "{eol}{open}#[|{close}]#{eol}",
                eol = LINE_END,
                open = pair.0,
                close = pair.1
            ),
            "i<backspace>",
            format!("{0}#[|{0}]#", LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_auto_pairs_disabled() -> anyhow::Result<()> {
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
                format!("{}#[|{}]#{}", pair.0, pair.1, LINE_END),
                "i<backspace>",
                format!("#[|{}]#{}", pair.1, LINE_END),
            ),
        )
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_before_multi_code_point_graphemes() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!("hello {}#[|👨‍👩‍👧‍👦]# goodbye{}", pair.1, LINE_END),
            "i<backspace>",
            format!("hello #[|👨‍👩‍👧‍👦]# goodbye{}", LINE_END),
        ))
        .await?;

        test((
            format!(
                "hello {}{}#[|👨‍👩‍👧‍👦]# goodbye{}",
                pair.0, pair.1, LINE_END
            ),
            "i<backspace>",
            format!("hello {}#[|👨‍👩‍👧‍👦]# goodbye{}", pair.0, LINE_END),
        ))
        .await?;

        test((
            format!(
                "hello {}#[|{}]#👨‍👩‍👧‍👦 goodbye{}",
                pair.0, pair.1, LINE_END
            ),
            "i<backspace>",
            format!("hello #[|👨‍👩‍👧‍👦]# goodbye{}", LINE_END),
        ))
        .await?;

        test((
            format!(
                "hello {}#[|{}👨‍👩‍👧‍👦]# goodbye{}",
                pair.0, pair.1, LINE_END
            ),
            "i<backspace>",
            format!("hello #[|👨‍👩‍👧‍👦]# goodbye{}", LINE_END),
        ))
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_at_end_of_document() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test(TestCase {
            in_text: format!("{}{}{}", LINE_END, pair.0, pair.1),
            in_selection: Selection::single(LINE_END.len() + 1, LINE_END.len() + 2),
            in_keys: String::from("i<backspace>"),
            out_text: String::from(LINE_END),
            out_selection: Selection::single(LINE_END.len(), LINE_END.len()),
        })
        .await?;

        test(TestCase {
            in_text: format!("foo{}{}{}", LINE_END, pair.0, pair.1),
            in_selection: Selection::single(LINE_END.len() + 4, LINE_END.len() + 5),
            in_keys: String::from("i<backspace>"),
            out_text: format!("foo{}", LINE_END),
            out_selection: Selection::single(3 + LINE_END.len(), 3 + LINE_END.len()),
        })
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_nested_open_inside_pair() -> anyhow::Result<()> {
    for pair in differing_pairs() {
        test((
            format!(
                "{open}{open}#[|{close}]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "i<backspace>",
            format!(
                "{open}#[|{close}]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_nested_open_inside_pair_multi() -> anyhow::Result<()> {
    for outer_pair in DEFAULT_PAIRS {
        for inner_pair in DEFAULT_PAIRS {
            if inner_pair.0 == outer_pair.0 {
                continue;
            }

            test((
                format!(
                    "{outer_open}{inner_open}#[|{inner_close}]#{outer_close}{eol}{outer_open}{inner_open}#(|{inner_close})#{outer_close}{eol}{outer_open}{inner_open}#(|{inner_close})#{outer_close}{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    inner_open = inner_pair.0,
                    inner_close = inner_pair.1,
                    eol = LINE_END
                ),
                "i<backspace>",
                format!(
                    "{outer_open}#[|{outer_close}]#{eol}{outer_open}#(|{outer_close})#{eol}{outer_open}#(|{outer_close})#{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    eol = LINE_END
                ),
            ))
            .await?;
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_basic() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "#[{eol}{open}|]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "a<backspace>",
            format!("#[{eol}{eol}|]#", eol = LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_multi_range() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "#[ {open}|]#{close}{eol}#( {open}|)#{close}{eol}#( {open}|)#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "a<backspace>",
            format!("#[ {eol}|]##( {eol}|)##( {eol}|)#", eol = LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_end_of_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "fo#[o{open}|]#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "a<backspace>",
            format!("fo#[o{}|]#", LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_mixed_dedent() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    bar = {}#[|{}]#
                        #(|\n)#
                    foo#(|\n)#
                "},
                pair.0, pair.1,
            )),
            "i<backspace>",
            helpers::platform_line(indoc! {"\
                bar = #[|\n]#
                #(|\n)#
                fo#(|\n)#
            "}),
        ))
        .await?;

        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    bar = {}#[|{}woop]#
                        #(|word)#
                    fo#(|o)#
                "},
                pair.0, pair.1,
            )),
            "i<backspace>",
            helpers::platform_line(indoc! {"\
                bar = #[|woop]#
                #(|word)#
                f#(|o)#
            "}),
        ))
        .await?;

        // delete from the right with append
        test((
            helpers::platform_line(&format!(
                indoc! {"\
                    bar = #[|woop{}]#{}
                    #(|    )#word
                    #(|fo)#o
                "},
                pair.0, pair.1,
            )),
            "a<backspace>",
            helpers::platform_line(indoc! {"\
                bar = #[woop\n|]#
                #(w|)#ord
                #(fo|)#
            "}),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_end_of_word_multi() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "fo#[o{open}|]#{close}{eol}fo#(o{open}|)#{close}{eol}fo#(o{open}|)#{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "a<backspace>",
            format!("fo#[o{eol}|]#fo#(o{eol}|)#fo#(o{eol}|)#", eol = LINE_END),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_inside_nested_pair() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "f#[oo{open}{open}|]#{close}{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "a<backspace>",
            format!(
                "f#[oo{open}{close}|]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_middle_of_word() -> anyhow::Result<()> {
    for pair in DEFAULT_PAIRS {
        test((
            format!(
                "f#[oo{open}{open}|]#{close}{close}{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
            "a<backspace>",
            format!(
                "f#[oo{open}{close}|]#{eol}",
                open = pair.0,
                close = pair.1,
                eol = LINE_END
            ),
        ))
        .await?;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_append_inside_nested_pair_multi() -> anyhow::Result<()> {
    for outer_pair in DEFAULT_PAIRS {
        for inner_pair in DEFAULT_PAIRS {
            if inner_pair.0 == outer_pair.0 {
                continue;
            }

            test((
                format!(
                    "f#[oo{outer_open}{inner_open}|]#{inner_close}{outer_close}{eol}f#(oo{outer_open}{inner_open}|)#{inner_close}{outer_close}{eol}f#(oo{outer_open}{inner_open}|)#{inner_close}{outer_close}{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    inner_open = inner_pair.0,
                    inner_close = inner_pair.1,
                    eol = LINE_END
                ),
                "a<backspace>",
                format!(
                    "f#[oo{outer_open}{outer_close}|]#{eol}f#(oo{outer_open}{outer_close}|)#{eol}f#(oo{outer_open}{outer_close}|)#{eol}",
                    outer_open = outer_pair.0,
                    outer_close = outer_pair.1,
                    eol = LINE_END
                ),
            ))
            .await?;
        }
    }

    Ok(())
}
