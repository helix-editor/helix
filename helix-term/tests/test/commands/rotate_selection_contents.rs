use super::*;

// Progression: A -> B -> C -> D
//              as we press `A-)`
const A: &str = indoc! {"
    #(a|)#
    #(b|)#
    #(c|)#
    #[d|]#
    #(e|)#"
};

const B: &str = indoc! {"
    #(e|)#
    #(a|)#
    #(b|)#
    #(c|)#
    #[d|]#"
};

const C: &str = indoc! {"
    #[d|]#
    #(e|)#
    #(a|)#
    #(b|)#
    #(c|)#"
};

const D: &str = indoc! {"
    #(c|)#
    #[d|]#
    #(e|)#
    #(a|)#
    #(b|)#"
};

#[tokio::test(flavor = "multi_thread")]
async fn rotate_selection_contents_forward_repeated() -> anyhow::Result<()> {
    test((A, "<A-)>", B)).await?;
    test((B, "<A-)>", C)).await?;
    test((C, "<A-)>", D)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn rotate_selection_contents_backward_repeated() -> anyhow::Result<()> {
    test((D, "<A-(>", C)).await?;
    test((C, "<A-(>", B)).await?;
    test((B, "<A-(>", A)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn rotate_selection_contents_in_groups() -> anyhow::Result<()> {
    // Count specifies the group size
    test((
        indoc! {"
            #(a|)#: #(1|)#
            #[b|]#: #(2|)#
            #(c|)#: #(3|)#
        "},
        "2<A-)>",
        indoc! {"
            #(1|)#: #(a|)#
            #(2|)#: #[b|]#
            #(3|)#: #(c|)#
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn rotate_selection_contents_with_trailing_group() -> anyhow::Result<()> {
    // When the amount of selections isn't divisible by the group size, the
    // last group is treated as it's own group, the primary selection should
    // stay put in the case below
    test((
        indoc! {"
            #(a|)#: #(1|)#
            #(b|)#: #(2|)#
            #[c|]#
        "},
        "2<A-)>",
        indoc! {"
            #(1|)#: #(a|)#
            #(2|)#: #(b|)#
            #[c|]#
        "},
    ))
    .await?;

    Ok(())
}
