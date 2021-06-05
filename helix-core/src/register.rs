use crate::Tendril;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};

// TODO: could be an instance on Editor
static REGISTRY: Lazy<RwLock<HashMap<char, Vec<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

const SYSTEM_CLIPBOARD_REGISTER: char = '+';

// TODO: we need a way to hook to system clipboard events to replace "+ register value
// when appropriate
pub fn system_clipboard_updated(content: String) {
    REGISTRY
        .write()
        .unwrap()
        .insert(SYSTEM_CLIPBOARD_REGISTER, vec![content]);
}

/// Read register values.
pub fn get(register_name: char) -> Option<Vec<String>> {
    let registry = REGISTRY.read().unwrap();
    // TODO: no cloning
    registry.get(&register_name).cloned()
}

/// Store values into the register.
/// `+` is a special register interfaced with system clipboard.
// restoring: bool
pub fn set(register_name: char, values: Vec<String>) {
    if register_name == SYSTEM_CLIPBOARD_REGISTER {
        let _ = set_clipboard_content(values.join("\n"));
    }

    let mut registry = REGISTRY.write().unwrap();
    registry.insert(register_name, values);
}

fn set_clipboard_content(
    value: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use copypasta::{ClipboardContext, ClipboardProvider};
    ClipboardContext::new()?.set_contents(value)
}
