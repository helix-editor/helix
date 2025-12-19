use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::config_serde_adapter;
use crate::OptionRegistry;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineNumber {
    /// Show absolute line number
    #[serde(alias = "abs")]
    Absolute,
    /// If focused and in normal/select mode, show relative line number to the primary cursor.
    /// If unfocused or in insert mode, show absolute line number.
    #[serde(alias = "rel")]
    Relative,
}

config_serde_adapter!(LineNumber);

fn setup_registry() -> OptionRegistry {
    let mut registry = OptionRegistry::new();
    registry.register(
        "scrolloff",
        "Number of lines of padding around the edge of the screen when scrolling",
        5usize,
    );
    registry.register(
        "shell",
        "Shell to use when running external commands",
        &["sh", "-c"],
    );
    registry.register("mouse", "Enable mouse mode", true);
    registry.register(
        "line-number",
        "Line number display: `absolute` simply shows each line's number, while \
        `relative` shows the distance from the current line. When unfocused or in \
        insert mode, `relative` will still show absolute line numbers",
        LineNumber::Absolute,
    );
    registry
}

#[test]
fn default_values() {
    let registry = setup_registry();
    let global_scope = registry.global_scope();
    let scrolloff: usize = *global_scope.get("scrolloff");
    let shell_ = global_scope.get_deref::<Box<[_]>>("shell");
    let shell: &[Box<str>] = &shell_;
    let mouse: bool = *global_scope.get("mouse");
    let line_number: LineNumber = *global_scope.get("line-number");
    assert_eq!(scrolloff, 5);
    assert!(shell.iter().map(Box::deref).eq(["sh", "-c"]));
    assert!(mouse);
    assert_eq!(line_number, LineNumber::Absolute);
}

#[test]
fn scope_overwrite() {
    let registry = setup_registry();
    let global_scope = registry.global_scope();
    let scope_1 = Arc::new(global_scope.create_scope());
    let scope_2 = Arc::new(global_scope.create_scope());
    let mut scope_3 = scope_1.create_scope();
    scope_1.set("line-number", "rel", &registry).unwrap();
    let line_number: LineNumber = *scope_3.get("line-number");
    assert_eq!(line_number, LineNumber::Relative);
    scope_3.set_parent_scope(scope_2.clone());
    let line_number: LineNumber = *scope_3.get("line-number");
    assert_eq!(line_number, LineNumber::Absolute);
    scope_2.set("line-number", "rel", &registry).unwrap();
    let line_number: LineNumber = *scope_3.get("line-number");
    assert_eq!(line_number, LineNumber::Relative);
    scope_2.set("line-number", "abs", &registry).unwrap();
    let line_number: LineNumber = *scope_3.get("line-number");
    assert_eq!(line_number, LineNumber::Absolute);
}
