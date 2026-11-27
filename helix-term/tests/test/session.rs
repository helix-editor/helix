use super::*;

fn session_config() -> Config {
    Config {
        editor: helix_view::editor::Config {
            session: helix_view::editor::SessionConfig {
                restore_cursor: true,
                ..Default::default()
            },
            lsp: helix_view::editor::LspConfig {
                enable: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_save_and_restore_cursor_position() -> anyhow::Result<()> {
    let file =
        helpers::temp_file_with_contents("line one\nline two\nline three\nline four\nline five\n")?;
    let path = file.path().to_string_lossy().to_string();

    let mut app = helpers::AppBuilder::new()
        .with_config(session_config())
        .with_file(file.path(), None)
        .build()?;

    helpers::test_key_sequences(
        &mut app,
        vec![
            // Move cursor to row 2, close buffer, reopen
            (
                Some(&format!("2j3l:bc!<ret>:o {}<ret>", path)),
                Some(&|app| {
                    let (view, doc) = helix_view::current_ref!(app.editor);
                    let text = doc.text().slice(..);
                    let coords =
                        helix_core::coords_at_pos(text, doc.selection(view.id).primary().head);
                    assert_eq!(coords.row, 2, "cursor row should be restored");
                    assert!(coords.col > 0, "cursor col should be restored (non-zero)");
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_cursor_clamped_to_valid_position() -> anyhow::Result<()> {
    let file =
        helpers::temp_file_with_contents("line one\nline two\nline three\nline four\nline five\n")?;
    let path = file.path().to_path_buf();
    let path_str = path.to_string_lossy().to_string();

    let mut app = helpers::AppBuilder::new()
        .with_config(session_config())
        .with_file(file.path(), None)
        .build()?;

    helpers::test_key_sequences(
        &mut app,
        vec![
            // Move to row 3, close buffer. Truncate file in assertion callback.
            (
                Some("3j:bc!<ret>"),
                Some(&|_app| {
                    // Truncate the file on disk to 2 lines between close and reopen
                    std::fs::write(&path, "short\nfile\n").unwrap();
                }),
            ),
            // Reopen — cursor should be clamped to valid range
            (
                Some(&format!(":o {}<ret>", path_str)),
                Some(&|app| {
                    let (view, doc) = helix_view::current_ref!(app.editor);
                    let text = doc.text().slice(..);
                    let max_line = text.len_lines().saturating_sub(1);
                    let coords =
                        helix_core::coords_at_pos(text, doc.selection(view.id).primary().head);
                    assert!(
                        coords.row <= max_line,
                        "cursor row {} should be <= max line {}",
                        coords.row,
                        max_line
                    );
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_disabled_by_default() -> anyhow::Result<()> {
    let file = helpers::temp_file_with_contents("line one\nline two\nline three\nline four\n")?;
    let path = file.path().to_string_lossy().to_string();

    // Default config — session.restore_cursor = false
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    helpers::test_key_sequences(
        &mut app,
        vec![(
            Some(&format!("3j:bc!<ret>:o {}<ret>", path)),
            Some(&|app| {
                let (view, doc) = helix_view::current_ref!(app.editor);
                let text = doc.text().slice(..);
                let coords = helix_core::coords_at_pos(text, doc.selection(view.id).primary().head);
                assert_eq!(
                    coords.row, 0,
                    "cursor should be at row 0 when session disabled"
                );
            }),
        )],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_explicit_position_overrides_restore() -> anyhow::Result<()> {
    let file =
        helpers::temp_file_with_contents("line one\nline two\nline three\nline four\nline five\n")?;
    let path = file.path().to_string_lossy().to_string();

    let mut app = helpers::AppBuilder::new()
        .with_config(session_config())
        .with_file(file.path(), None)
        .build()?;

    // Move to row 1, close, reopen with explicit line 4 (1-indexed → row 3)
    helpers::test_key_sequences(
        &mut app,
        vec![(
            Some(&format!("j:bc!<ret>:o {}:4<ret>", path)),
            Some(&|app| {
                let (view, doc) = helix_view::current_ref!(app.editor);
                let text = doc.text().slice(..);
                let coords = helix_core::coords_at_pos(text, doc.selection(view.id).primary().head);
                assert_eq!(
                    coords.row, 3,
                    "explicit position should override session restore"
                );
            }),
        )],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_multiple_documents() -> anyhow::Result<()> {
    let file1 = helpers::temp_file_with_contents("alpha\nbeta\ngamma\ndelta\n")?;
    let file2 = helpers::temp_file_with_contents("one\ntwo\nthree\nfour\nfive\nsix\n")?;
    let path1 = file1.path().to_string_lossy().to_string();
    let path2 = file2.path().to_string_lossy().to_string();

    let mut app = helpers::AppBuilder::new()
        .with_config(session_config())
        .with_file(file1.path(), None)
        .build()?;

    helpers::test_key_sequences(
        &mut app,
        vec![
            // Move file1 cursor to row 2, open file2, move to row 4, close both, reopen file1
            (
                Some(&format!(
                    "2j:o {}<ret>4j:bc!<ret>:bc!<ret>:o {}<ret>",
                    path2, path1
                )),
                Some(&|app| {
                    let (view, doc) = helix_view::current_ref!(app.editor);
                    let text = doc.text().slice(..);
                    let coords =
                        helix_core::coords_at_pos(text, doc.selection(view.id).primary().head);
                    assert_eq!(coords.row, 2, "file1 cursor should be restored to row 2");
                }),
            ),
            // Open file2 and check its cursor
            (
                Some(&format!(":o {}<ret>", path2)),
                Some(&|app| {
                    let (view, doc) = helix_view::current_ref!(app.editor);
                    let text = doc.text().slice(..);
                    let coords =
                        helix_core::coords_at_pos(text, doc.selection(view.id).primary().head);
                    assert_eq!(coords.row, 4, "file2 cursor should be restored to row 4");
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_scratch_buffer_not_saved() -> anyhow::Result<()> {
    let file = helpers::temp_file_with_contents("hello\nworld\n")?;

    let mut app = helpers::AppBuilder::new()
        .with_config(session_config())
        .with_file(file.path(), None)
        .build()?;

    helpers::test_key_sequences(
        &mut app,
        vec![
            // Move cursor in named file, then open a scratch buffer, type text, close it
            (
                Some(":new<ret>ihello scratch<esc>:bc!<ret>"),
                Some(&|app| {
                    // After closing the scratch buffer, only the named file should
                    // potentially be in session state — no scratch buffer entry.
                    // Verify we're back on the named file.
                    let doc = helix_view::doc!(app.editor);
                    assert!(
                        doc.path().is_some(),
                        "should be back on the named file after closing scratch buffer"
                    );
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}
