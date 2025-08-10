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
async fn rotate_selection_contents_forward_with_count() -> anyhow::Result<()> {
    test((A, "2<A-)>", C)).await?;
    test((A, "3<A-)>", D)).await?;
    test((B, "2<A-)>", D)).await?;

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
async fn rotate_selection_contents_backward_with_count() -> anyhow::Result<()> {
    test((D, "2<A-(>", B)).await?;
    test((D, "3<A-(>", A)).await?;
    test((C, "2<A-(>", A)).await?;

    Ok(())
}
