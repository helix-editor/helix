use super::helpers::*;
use helix_term::config::ConfigRaw;

fn build_app(config_toml: toml::Table) -> anyhow::Result<helix_term::application::Application> {
    let config_raw: ConfigRaw = config_toml.try_into()?;

    let config = helix_term::config::Config {
        theme: config_raw.theme,
        keys: config_raw.keys.unwrap(),
        editor: config_raw.editor.unwrap().try_into()?,
    };

    let app = AppBuilder::new().with_config(config).build()?;

    Ok(app)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_set_config_single() -> anyhow::Result<()> {
    let mut app = build_app(toml::toml! {
        editor.trim-final-newlines = false
        editor.trim-trailing-whitespace = false
        keys.normal.space.t = ":set trim-trailing-whitespace true"
    })?;

    test_key_sequence(
        &mut app,
        Some("<space>t"),
        Some(&|app| {
            let config = app.editor.config.load();
            assert!(!config.trim_final_newlines);
            assert!(config.trim_trailing_whitespace);
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_set_config_multi() -> anyhow::Result<()> {
    let mut app = build_app(toml::toml! {
        editor.trim-final-newlines = false
        editor.trim-trailing-whitespace = false
        keys.normal.space.t = [
            ":set trim-final-newlines true",
            ":set trim-trailing-whitespace true",
        ]
    })?;

    test_key_sequence(
        &mut app,
        Some("<space>t"),
        Some(&|app| {
            let config = app.editor.config.load();
            assert!(config.trim_final_newlines);
            assert!(config.trim_trailing_whitespace);
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_toggle_config_single() -> anyhow::Result<()> {
    let mut app = build_app(toml::toml! {
        editor.trim-final-newlines = false
        editor.trim-trailing-whitespace = false
        keys.normal.space.t = ":toggle trim-trailing-whitespace"
    })?;

    test_key_sequence(
        &mut app,
        Some("<space>t"),
        Some(&|app| {
            let config = app.editor.config.load();
            assert!(!config.trim_final_newlines);
            assert!(config.trim_trailing_whitespace);
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_toggle_config_multi() -> anyhow::Result<()> {
    let mut app = build_app(toml::toml! {
        editor.trim-final-newlines = false
        editor.trim-trailing-whitespace = false
        keys.normal.space.t = [
            ":toggle trim-final-newlines",
            ":toggle trim-trailing-whitespace",
        ]
    })?;

    test_key_sequence(
        &mut app,
        Some("<space>t"),
        Some(&|app| {
            let config = app.editor.config.load();
            assert!(config.trim_final_newlines);
            assert!(config.trim_trailing_whitespace);
        }),
        false,
    )
    .await?;

    Ok(())
}
