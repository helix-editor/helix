use super::*;

#[tokio::test]
async fn auto_pairs_basic() -> anyhow::Result<()> {
    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        ("#[\n|]#", "i(<esc>", "(#[|)]#\n"),
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config {
            editor: helix_view::editor::Config {
                auto_pairs: AutoPairConfig::Enable(false),
                ..Default::default()
            },
            ..Default::default()
        },
        ("#[\n|]#", "i(<esc>", "(#[|\n]#"),
    )?;

    Ok(())
}
