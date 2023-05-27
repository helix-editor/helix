use std::str::from_utf8;

/// Default built-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    let default_config = include_bytes!("../../languages.toml");
    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let config = [
        crate::config_dir(),
        crate::find_workspace().0.join(".helix"),
    ]
    .into_iter()
    .map(|path| path.join("languages.toml"))
    .filter_map(|file| {
        std::fs::read_to_string(file)
            .map(|config| toml::from_str(&config))
            .ok()
    })
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .fold(default_lang_config(), |a, b| {
        // combines for example
        // b:
        //   [[language]]
        //   name = "toml"
        //   language-server = { command = "taplo", args = ["lsp", "stdio"] }
        //
        // a:
        //   [[language]]
        //   language-server = { command = "/usr/bin/taplo" }
        //
        // into:
        //   [[language]]
        //   name = "toml"
        //   language-server = { command = "/usr/bin/taplo" }
        //
        // thus it overrides the third depth-level of b with values of a if they exist, but otherwise merges their values
        crate::merge_toml_values(a, b, 3)
    });

    Ok(config)
}
