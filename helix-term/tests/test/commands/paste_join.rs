use super::*;

const IN: &str = indoc! {"\
    #[o|]#ne
    #(t|)#wo
    three
"};

#[tokio::test(flavor = "multi_thread")]
async fn after() -> anyhow::Result<()> {
    const OUT: &str = indoc! {"\
        o#[o
        t|]#ne
        t#(o
        t|)#wo
        three
    "};

    test((IN, "y:paste-join<ret>", OUT)).await?;
    test((IN, "y<C-p>", OUT)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn before() -> anyhow::Result<()> {
    const OUT: &str = indoc! {"\
        #[o
        t|]#one
        #(o
        t|)#two
        three
    "};

    test((IN, "y:paste-join --position before<ret>", OUT)).await?;
    test((IN, "y<C-P>", OUT)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn replace() -> anyhow::Result<()> {
    const OUT: &str = indoc! {"\
            #[o
            t|]#ne
            #(o
            t|)#wo
            three
        "};

    test((IN, "y:paste-join --position replace<ret>", OUT)).await?;
    test((IN, "y<C-R>", OUT)).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn flag_count() -> anyhow::Result<()> {
    test((
        IN,
        "y:paste-join --count 4<ret>",
        indoc! {"\
            o#[o
            to
            to
            to
            t|]#ne
            t#(o
            to
            to
            to
            t|)#wo
            three
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn flag_register() -> anyhow::Result<()> {
    test((
        IN,
        concat!(
            // Copy content from another place, so our default
            // register has different content to what we will
            // have in register 'x'
            "jlykh",
            // Pasting from 'x' does not paste from the default register
            "\"xy:paste-join --register x<ret>"
        ),
        indoc! {"\
            o#[o
            t|]#ne
            t#(o
            t|)#wo
            three
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn flag_separator() -> anyhow::Result<()> {
    test((
        IN,
        "y:paste-join --separator x<ret>",
        indoc! {"\
            o#[oxt|]#ne
            t#(oxt|)#wo
            three
        "},
    ))
    .await?;

    Ok(())
}
