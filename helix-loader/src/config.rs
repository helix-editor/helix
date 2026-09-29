use std::str::from_utf8;

use crate::workspace_trust::{TrustQuery, WorkspaceTrust};

/// Default built-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    let default_config = include_bytes!("../../languages.toml");
    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
///
/// Workspace-local `.helix/languages.toml` is merged in only when the current
/// workspace is trusted for [`TrustQuery::LocalConfig`].
pub fn user_lang_config(trust: &WorkspaceTrust) -> Result<toml::Value, toml::de::Error> {
    let global_config = crate::lang_config_file();
    let workspace_config = crate::workspace_lang_config_file();

    let files = if trust.query_current(TrustQuery::LocalConfig).is_trusted() {
        vec![global_config, workspace_config]
    } else {
        vec![global_config]
    };

    let config = files
        .iter()
        .filter_map(|file| {
            std::fs::read_to_string(file)
                .map(|config| toml::from_str(&config))
                .ok()
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .fold(default_lang_config(), |a, b| {
            crate::merge_toml_values(a, b, 3)
        });

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_trust::{Config as TrustConfig, WorkspaceTrust};
    use std::{fs, sync::Mutex};

    /// `user_lang_config` resolves the workspace from the process working
    /// directory, which is global state: tests that change it must not run
    /// concurrently.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn workspace_lang_config_merged_only_when_trusted() {
        let _guard = CWD_LOCK.lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".helix")).unwrap();
        fs::write(
            dir.path().join(".helix").join("languages.toml"),
            r#"
[[language]]
name = "typescript"
language-servers = []
"#,
        )
        .unwrap();

        let old_cwd = helix_stdx::env::current_working_dir();
        helix_stdx::env::set_current_working_dir(dir.path()).unwrap();

        let typescript_servers = |config: &toml::Value| -> usize {
            config
                .get("language")
                .and_then(|v| v.as_array())
                .unwrap()
                .iter()
                .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("typescript"))
                .unwrap()
                .get("language-servers")
                .and_then(|v| v.as_array())
                .unwrap()
                .len()
        };

        // Untrusted workspace: the override must not be merged in.
        let trust = WorkspaceTrust::new(TrustConfig::default());
        assert!(
            typescript_servers(&user_lang_config(&trust).unwrap()) > 0,
            "untrusted workspace config must not override the default language servers"
        );

        // Workspace trusted through a persisted grant (as written by
        // `:workspace-trust`): the override must be merged in.
        trust.trust(dir.path());
        assert_eq!(
            typescript_servers(&user_lang_config(&trust).unwrap()),
            0,
            "trusted workspace config must override the default language servers"
        );

        helix_stdx::env::set_current_working_dir(old_cwd).unwrap();
    }
}
