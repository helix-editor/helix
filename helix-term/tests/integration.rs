#[cfg(feature = "integration")]
mod test {
    mod helpers;

    use std::path::PathBuf;

    use helix_core::{syntax::AutoPairConfig, Position, Selection};
    use helix_term::{args::Args, config::Config};

    use indoc::indoc;

    use self::helpers::*;

    #[tokio::test]
    async fn hello_world() -> anyhow::Result<()> {
        test(("#[\n|]#", "ihello world<esc>", "hello world#[|\n]#")).await?;
        Ok(())
    }

    mod auto_indent;
    mod auto_pairs;
    mod commands;
    mod movement;
    mod write;
}
