use crate::Tendril;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};

// TODO: could be an instance on Editor
static REGISTRY: Lazy<RwLock<HashMap<char, Vec<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Read register values.
pub fn get(register_name: char) -> Option<Vec<String>> {
    let registry = REGISTRY.read().unwrap();
    registry.get(&register_name).cloned() // TODO: no cloning
}

/// Read register values.
// restoring: bool
pub fn set(register_name: char, values: Vec<String>) {
    let mut registry = REGISTRY.write().unwrap();
    registry.insert(register_name, values);
}
