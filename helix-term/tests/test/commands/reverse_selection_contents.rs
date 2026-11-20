use super::*;

const A: &str = indoc! {"
    #(a|)#
    #(b|)#
    #(c|)#
    #[d|]#
    #(e|)#"
};
const A_REV: &str = indoc! {"
    #(e|)#
    #[d|]#
    #(c|)#
    #(b|)#
    #(a|)#"
};
const B: &str = indoc! {"
    #(a|)#
    #(b|)#
    #[c|]#
    #(d|)#
    #(e|)#"
};
const B_REV: &str = indoc! {"
    #(e|)#
    #(d|)#
    #[c|]#
    #(b|)#
    #(a|)#"
};

const CMD: &str = "<space>?reverse_selection_contents<ret>";

#[tokio::test(flavor = "multi_thread")]
async fn reverse_selection_contents() -> anyhow::Result<()> {
    test((A, CMD, A_REV)).await?;
    test((B, CMD, B_REV)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn reverse_selection_contents_in_groups() -> anyhow::Result<()> {
    test((
        indoc! {"
            #(a|)# | #(1|)# | #(x|)#
            #[b|]# | #(2|)# | #(y|)#
            #(c|)# | #(3|)# | #(z|)#
        "},
        format!("3{CMD}"),
        indoc! {"
            #(x|)# | #(1|)# | #(a|)#
            #(y|)# | #(2|)# | #[b|]#
            #(z|)# | #(3|)# | #(c|)#
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn reverse_selection_contents_with_trailing_group() -> anyhow::Result<()> {
    // When the amount of selections isn't divisible by the group size, the
    // last group is treated as it's own group, in this case the group (c, 3)
    // the primary selection should move correctly with the reversal
    test((
        indoc! {"
            #(a|)# | #(1|)# | #(x|)#
            #(b|)# | #(2|)# | #(y|)#
            #[c|]# | #(3|)#
        "},
        format!("3{CMD}"),
        indoc! {"
            #(x|)# | #(1|)# | #(a|)#
            #(y|)# | #(2|)# | #(b|)#
            #(3|)# | #[c|]#
        "},
    ))
    .await?;

    Ok(())
}
