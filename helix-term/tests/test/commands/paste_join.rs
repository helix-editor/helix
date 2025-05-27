use super::*;

const IN: &str = indoc! {"\
    #[o|]#ne
    #(t|)#wo
    three
"};

#[tokio::test(flavor = "multi_thread")]
async fn after() -> anyhow::Result<()> {
    test((
        IN,
        "y:paste-join<ret>",
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
async fn before() -> anyhow::Result<()> {
    test((
        IN,
        "y:paste-join --position before<ret>",
        indoc! {"\
            #[o
            t|]#one
            #(o
            t|)#two
            three
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn replace() -> anyhow::Result<()> {
    test((
        IN,
        "y:paste-join --position replace<ret>",
        indoc! {"\
            #[o
            t|]#ne
            #(o
            t|)#wo
            three
        "},
    ))
    .await?;

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
