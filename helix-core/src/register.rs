use crate::Tendril;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};

// TODO: could be an instance on Editor
static REGISTRY: Lazy<RwLock<HashMap<char, Vec<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn get(register: char) -> Option<Vec<String>> {
    let registry = REGISTRY.read().unwrap();

    // TODO: no cloning
    registry.get(&register).cloned()
}

// restoring: bool
pub fn set(register: char, values: Vec<String>) {
    let mut registry = REGISTRY.write().unwrap();

    registry.insert(register, values);
}
