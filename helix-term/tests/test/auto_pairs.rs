use super::*;

#[tokio::test]
async fn auto_pairs_basic() -> anyhow::Result<()> {
    test(("#[\n|]#", "i(<esc>", "(#[|)]#\n")).await?;

    test_with_config(
        Args::default(),
        Config {
            editor: helix_view::editor::Config {
                auto_pairs: AutoPairConfig::Enable(false),
                ..Default::default()
            },
            ..Default::default()
        },
        ("#[\n|]#", "i(<esc>", "(#[|\n]#"),
    )
    .await?;

    Ok(())
}
