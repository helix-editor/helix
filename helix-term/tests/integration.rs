#[cfg(feature = "integration")]
mod test {
    mod helpers;

    use helix_core::{syntax::AutoPairConfig, Selection};
    use helix_term::config::Config;

    use indoc::indoc;

    use self::helpers::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn hello_world() -> anyhow::Result<()> {
        test(("#[\n|]#", "ihello world<esc>", "hello world#[|\n]#")).await?;
        Ok(())
    }

    mod auto_indent;
    mod auto_pairs;
    mod command_line;
    mod commands;
    mod languages;
    mod movement;
    mod splits;
}
