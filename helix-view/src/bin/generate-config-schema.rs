use helix_view::editor::Config;
use schemars::{schema_for, JsonSchema};

pub fn main() {
    let schema = schema_for!(Config);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
