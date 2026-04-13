use super::*;

use helix_stdx::path;

#[tokio::test(flavor = "multi_thread")]
async fn test_diff_open_creates_session() -> anyhow::Result<()> {
    let file1 = helpers::temp_file_with_contents("one\ntwo\nthree\n")?;
    let file2 = helpers::temp_file_with_contents("one\nTWO\nthree\n")?;

    let mut app = helpers::AppBuilder::new().build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some(&format!(
                    ":diff-open {} {}<ret>",
                    file1.path().to_string_lossy(),
                    file2.path().to_string_lossy()
                )),
                Some(&|app| {
                    assert_eq!(1, app.editor.diff_sessions.len());
                    assert_eq!(2, app.editor.tree.views().count());
                    helpers::assert_status_not_error(&app.editor);

                    let norm1 = path::normalize(file1.path());
                    let norm2 = path::normalize(file2.path());
                    assert!(app.editor.documents().any(|d| d.path() == Some(&norm1)));
                    assert!(app.editor.documents().any(|d| d.path() == Some(&norm2)));
                }),
            ),
            (Some(":qa!<ret>"), None),
        ],
        true,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_diff_this_pairs_views() -> anyhow::Result<()> {
    let file1 = helpers::temp_file_with_contents("one\ntwo\nthree\n")?;
    let file2 = helpers::temp_file_with_contents("one\nTWO\nthree\n")?;

    let mut app = helpers::AppBuilder::new()
        .with_file(file1.path(), None)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                // Open file2 in a split, then mark it as the first diff view.
                Some(&format!(
                    ":sp<ret>:o {}<ret>:diff-this<ret>",
                    file2.path().to_string_lossy()
                )),
                Some(&|app| {
                    assert!(app.editor.pending_diff_this.is_some());
                    assert_eq!(0, app.editor.diff_sessions.len());
                    helpers::assert_status_not_error(&app.editor);
                }),
            ),
            (
                // Switch to the other view and pair it.
                Some("<C-w>w:diff-this<ret>"),
                Some(&|app| {
                    assert_eq!(1, app.editor.diff_sessions.len());
                    assert!(app.editor.pending_diff_this.is_none());
                    helpers::assert_status_not_error(&app.editor);
                }),
            ),
            (Some(":qa!<ret>"), None),
        ],
        true,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_diff_off_removes_session() -> anyhow::Result<()> {
    let file1 = helpers::temp_file_with_contents("one\ntwo\nthree\n")?;
    let file2 = helpers::temp_file_with_contents("one\nTWO\nthree\n")?;

    let mut app = helpers::AppBuilder::new().build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some(&format!(
                    ":diff-open {} {}<ret>",
                    file1.path().to_string_lossy(),
                    file2.path().to_string_lossy()
                )),
                Some(&|app| {
                    assert_eq!(1, app.editor.diff_sessions.len());
                    assert_eq!(2, app.editor.tree.views().count());
                }),
            ),
            (
                Some(":diff-off<ret>"),
                Some(&|app| {
                    assert!(app.editor.diff_sessions.is_empty());
                    // Both views remain open after the session ends.
                    assert_eq!(2, app.editor.tree.views().count());
                    helpers::assert_status_not_error(&app.editor);
                }),
            ),
            (Some(":qa!<ret>"), None),
        ],
        true,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_diffget_pulls_hunk_from_partner() -> anyhow::Result<()> {
    // left: "two", right: "TWO". After diffget from view A, left becomes "TWO".
    let file1 = helpers::temp_file_with_contents("one\ntwo\nthree\n")?;
    let file2 = helpers::temp_file_with_contents("one\nTWO\nthree\n")?;

    let mut app = helpers::AppBuilder::new().build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                // After diff-open, focus is on view B (right/file2).
                // <C-w>w switches to view A (left/file1). j moves to the changed line.
                Some(&format!(
                    ":diff-open {} {}<ret><C-w>wj:diffget<ret>",
                    file1.path().to_string_lossy(),
                    file2.path().to_string_lossy()
                )),
                Some(&|app| {
                    helpers::assert_status_not_error(&app.editor);
                    assert_eq!(1, app.editor.diff_sessions.len());

                    let doc_a_id = app.editor.diff_sessions[0].doc_a();
                    let doc_text = app.editor.documents[&doc_a_id].text().to_string();
                    assert_eq!("one\nTWO\nthree\n", doc_text);
                }),
            ),
            (Some(":qa!<ret>"), None),
        ],
        true,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_diffput_pushes_hunk_to_partner() -> anyhow::Result<()> {
    // left: "two", right: "TWO". After diffput from view A, right becomes "two".
    let file1 = helpers::temp_file_with_contents("one\ntwo\nthree\n")?;
    let file2 = helpers::temp_file_with_contents("one\nTWO\nthree\n")?;

    let mut app = helpers::AppBuilder::new().build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some(&format!(
                    ":diff-open {} {}<ret><C-w>wj:diffput<ret>",
                    file1.path().to_string_lossy(),
                    file2.path().to_string_lossy()
                )),
                Some(&|app| {
                    helpers::assert_status_not_error(&app.editor);
                    assert_eq!(1, app.editor.diff_sessions.len());

                    let doc_b_id = app.editor.diff_sessions[0].doc_b();
                    let doc_text = app.editor.documents[&doc_b_id].text().to_string();
                    assert_eq!("one\ntwo\nthree\n", doc_text);
                }),
            ),
            (Some(":qa!<ret>"), None),
        ],
        true,
    )
    .await
}
