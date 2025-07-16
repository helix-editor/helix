use helix_plugin::manager::PluginManager;
use std::path::PathBuf;

#[test]
fn test_plugin_discovery() {
    let mut manager = PluginManager::new();
    let test_plugins_dir = PathBuf::from("../tests/test-plugins");

    manager.discover_plugins_in(&test_plugins_dir).unwrap();

    assert_eq!(manager.plugins.len(), 1);
    let manifest = &manager.plugins[0];
    assert_eq!(manifest.name, "my-first-plugin");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.activation.on_command, vec!["my-plugin:test-command"]);
}
