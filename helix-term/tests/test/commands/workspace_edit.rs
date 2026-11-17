use std::path::Path;

use helix_lsp::{lsp, OffsetEncoding};
use helix_term::application::Application;
use serde_json::json;

use super::*;

fn apply(app: &mut Application, ops: serde_json::Value) -> bool {
    let edit: lsp::WorkspaceEdit =
        serde_json::from_value(json!({ "documentChanges": ops })).unwrap();
    app.editor
        .apply_workspace_edit(OffsetEncoding::Utf16, &edit)
        .is_ok()
}

fn uri(path: &Path) -> String {
    lsp::Url::from_file_path(path).unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn resource_ops_keep_existing_target() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let from = dir.path().join("from.txt");
    let to = dir.path().join("to.txt");
    std::fs::write(&from, "from")?;
    std::fs::write(&to, "keep")?;
    let mut app = AppBuilder::new().build()?;

    assert!(!apply(
        &mut app,
        json!([{ "kind": "create", "uri": uri(&to) }])
    ));
    assert!(!apply(
        &mut app,
        json!([{ "kind": "rename", "oldUri": uri(&from), "newUri": uri(&to) }])
    ));
    assert!(apply(
        &mut app,
        json!([{ "kind": "create", "uri": uri(&to), "options": { "ignoreIfExists": true } }])
    ));
    assert_eq!(std::fs::read_to_string(&to)?, "keep");
    assert!(from.exists());

    test_key_sequence(&mut app, None, None, false).await
}
