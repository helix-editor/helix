use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn insert_keymap_suffix() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_config(config()),
        ("#[|]#", "iselffd", "self#[|]#"),
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_keymap_suffix_non_char() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_config(config()),
        ("#[|]#", "i<F1>ua", "a#[|]#"),
    )
    .await?;

    Ok(())
}

fn config() -> Config {
    let config = r#"
        [keys.insert]
        f.d = "normal_mode"
        F1.j = "insert_newline"
    "#;
    Config::load(
        Ok(config.to_owned()),
        Err(helix_term::config::ConfigLoadError::default()),
    )
    .unwrap()
}
