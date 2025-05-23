use helix_core::{hashmap, NATIVE_LINE_ENDING};
use helix_term::keymap;
use helix_view::{doc, document::Mode};

use super::*;

// Tests being able to jump in insert mode, then undo the write performed by the jump
// https://github.com/helix-editor/helix/issues/13480
#[tokio::test(flavor = "multi_thread")]
async fn test_jump_undo() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.keys.insert(
        Mode::Insert,
        keymap!({"Insert Mode"
            "C-i" => goto_file_start,
        }),
    );
    let mut app = helpers::AppBuilder::new().with_config(config).build()?;

    test_key_sequence(
        &mut app,
        Some("iworld<C-i>Hello, <esc>u"),
        Some(&|app| {
            assert!(!app.editor.is_err());
            let doc = doc!(app.editor);
            assert_eq!(
                format!("world{}", NATIVE_LINE_ENDING.as_str()),
                doc.text().to_string()
            );
        }),
        false,
    )
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_jump_redo() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.keys.insert(
        Mode::Insert,
        keymap!({"Insert Mode"
            "C-i" => goto_file_start,
            "C-o" => goto_file_end,
        }),
    );
    let mut app = helpers::AppBuilder::new().with_config(config).build()?;

    test_key_sequence(
        &mut app,
        Some("iworld<C-i>Hello, <esc>ui<C-o><esc>U"),
        Some(&|app| {
            assert!(!app.editor.is_err());
            let doc = doc!(app.editor);
            assert_eq!(
                format!("Hello, world{}", NATIVE_LINE_ENDING.as_str()),
                doc.text().to_string()
            );
        }),
        false,
    )
    .await?;
    Ok(())
}
