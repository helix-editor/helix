use super::*;

use helix_stdx::path;

#[tokio::test(flavor = "multi_thread")]
async fn test_split_write_quit_all() -> anyhow::Result<()> {
    let mut file1 = tempfile::NamedTempFile::new()?;
    let mut file2 = tempfile::NamedTempFile::new()?;
    let mut file3 = tempfile::NamedTempFile::new()?;

    let mut app = helpers::AppBuilder::new()
        .with_file(file1.path(), None)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some(&format!(
                    "ihello1<esc>:sp<ret>:o {}<ret>ihello2<esc>:sp<ret>:o {}<ret>ihello3<esc>",
                    file2.path().to_string_lossy(),
                    file3.path().to_string_lossy()
                )),
                Some(&|app| {
                    let docs: Vec<_> = app.editor.documents().collect();
                    assert_eq!(3, docs.len());

                    let doc1 = docs
                        .iter()
                        .find(|doc| doc.path().unwrap() == &path::normalize(file1.path()))
                        .unwrap();

                    assert_eq!("hello1", doc1.text().to_string());

                    let doc2 = docs
                        .iter()
                        .find(|doc| doc.path().unwrap() == &path::normalize(file2.path()))
                        .unwrap();

                    assert_eq!("hello2", doc2.text().to_string());

                    let doc3 = docs
                        .iter()
                        .find(|doc| doc.path().unwrap() == &path::normalize(file3.path()))
                        .unwrap();

                    assert_eq!("hello3", doc3.text().to_string());

                    helpers::assert_status_not_error(&app.editor);
                    assert_eq!(3, app.editor.tree.views().count());
                }),
            ),
            (
                Some(":wqa<ret>"),
                Some(&|app| {
                    helpers::assert_status_not_error(&app.editor);
                    assert_eq!(0, app.editor.tree.views().count());
                }),
            ),
        ],
        true,
    )
    .await?;

    helpers::assert_file_has_content(&mut file1, &LineFeedHandling::Native.apply("hello1"))?;
    helpers::assert_file_has_content(&mut file2, &LineFeedHandling::Native.apply("hello2"))?;
    helpers::assert_file_has_content(&mut file3, &LineFeedHandling::Native.apply("hello3"))?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_split_write_quit_same_file() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some("O<esc>ihello<esc>:sp<ret>ogoodbye<esc>"),
                Some(&|app| {
                    assert_eq!(2, app.editor.tree.views().count());
                    helpers::assert_status_not_error(&app.editor);

                    let mut docs: Vec<_> = app.editor.documents().collect();
                    assert_eq!(1, docs.len());

                    let doc = docs.pop().unwrap();

                    assert_eq!(
                        LineFeedHandling::Native.apply("hello\ngoodbye"),
                        doc.text().to_string()
                    );

                    assert!(doc.is_modified());
                }),
            ),
            (
                Some(":wq<ret>"),
                Some(&|app| {
                    helpers::assert_status_not_error(&app.editor);
                    assert_eq!(1, app.editor.tree.views().count());

                    let mut docs: Vec<_> = app.editor.documents().collect();
                    assert_eq!(1, docs.len());

                    let doc = docs.pop().unwrap();

                    assert_eq!(
                        LineFeedHandling::Native.apply("hello\ngoodbye"),
                        doc.text().to_string()
                    );

                    assert!(!doc.is_modified());
                }),
            ),
        ],
        false,
    )
    .await?;

    helpers::assert_file_has_content(&mut file, &LineFeedHandling::Native.apply("hello\ngoodbye"))?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_changes_in_splits_apply_to_all_views() -> anyhow::Result<()> {
    // See <https://github.com/helix-editor/helix/issues/4732>.
    // Transactions must be applied to any view that has the changed document open.
    // This sequence would panic since the jumplist entry would be modified in one
    // window but not the other. Attempting to update the changelist in the other
    // window would cause a panic since it would point outside of the document.

    // The key sequence here:
    // * <C-w>v       Create a vertical split of the current buffer.
    //                Both views look at the same doc.
    // * [<space>     Add a line ending to the beginning of the document.
    //                The cursor is now at line 2 in window 2.
    // * <C-s>        Save that selection to the jumplist in window 2.
    // * <C-w>w       Switch to window 1.
    // * kd           Delete line 1 in window 1.
    // * <C-w>q       Close window 1, focusing window 2.
    // * d            Delete line 1 in window 2.
    //
    // This panicked in the past because the jumplist entry on line 2 of window 2
    // was not updated and after the `kd` step, pointed outside of the document.
    test((
        "#[|]#",
        "<C-w>v[<space><C-s><C-w>wkd<C-w>qd",
        "#[|]#",
        LineFeedHandling::AsIs,
    ))
    .await?;

    // Transactions are applied to the views for windows lazily when they are focused.
    // This case panics if the transactions and inversions are not applied in the
    // correct order as we switch between windows.
    test((
        "#[|]#",
        "[<space>[<space>[<space><C-w>vuuu<C-w>wUUU<C-w>quuu",
        "#[|]#",
        LineFeedHandling::AsIs,
    ))
    .await?;

    // See <https://github.com/helix-editor/helix/issues/4957>.
    // This sequence undoes part of the history and then adds new changes, creating a
    // new branch in the history tree. `View::sync_changes` applies transactions down
    // and up to the lowest common ancestor in the path between old and new revision
    // numbers. If we apply these up/down transactions in the wrong order, this case
    // panics.
    // The key sequence:
    // * 3[<space>    Create three empty lines so we are at the end of the document.
    // * <C-w>v<C-s>  Create a split and save that point at the end of the document
    //                in the jumplist.
    // * <C-w>w       Switch back to the first window.
    // * uu           Undo twice (not three times which would bring us back to the
    //                root of the tree).
    // * 3[<space>    Create three empty lines. Now the end of the document is past
    //                where it was on step 1.
    // * <C-w>q       Close window 1, focusing window 2 and causing a sync. This step
    //                panics if we don't apply in the right order.
    // * %d           Clean up the buffer.
    test((
        "#[|]#",
        "3[<space><C-w>v<C-s><C-w>wuu3[<space><C-w>q%d",
        "#[|]#",
        LineFeedHandling::AsIs,
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_regex_prompt_abort_after_view_switch_does_not_panic() -> anyhow::Result<()> {
    // See <https://github.com/helix-editor/helix/issues/13325>.
    //
    // Opening a regex prompt captures a selection snapshot for the focused
    // (view, doc) pair. The prompt does not consume mouse events, so a click
    // into a different view can shift focus without aborting the prompt.
    // Aborting (or updating) the prompt afterwards must not restore the
    // snapshot into the now-current document, since the snapshot's char
    // indices may exceed the new document's length and panic inside
    // `Selection::ensure_invariants`.
    use helix_view::input::parse_macro;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    #[cfg(windows)]
    use crossterm::event::KeyModifiers as TerminalKeyModifiers;
    #[cfg(windows)]
    use crossterm::event::{
        Event as TerminalEvent, KeyEvent as TerminalKey, MouseButton, MouseEvent, MouseEventKind,
    };
    #[cfg(not(windows))]
    use termina::event::Modifiers as TerminalKeyModifiers;
    #[cfg(not(windows))]
    use termina::event::{
        Event as TerminalEvent, KeyEvent as TerminalKey, MouseButton, MouseEvent, MouseEventKind,
    };

    let file1 = tempfile::NamedTempFile::new()?;
    let file2 = tempfile::NamedTempFile::new()?;

    let mut app = AppBuilder::new().with_file(file1.path(), None).build()?;

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rx_stream = UnboundedReceiverStream::new(rx);

    let send_keys = |tx: &tokio::sync::mpsc::UnboundedSender<std::io::Result<TerminalEvent>>,
                     macro_str: &str|
     -> anyhow::Result<()> {
        for key_event in parse_macro(macro_str)?.into_iter() {
            let key = TerminalEvent::Key(TerminalKey::from(key_event));
            tx.send(Ok(key))?;
        }
        Ok(())
    };

    // Populate file1 with three lines, vertical split, open file2 in the new
    // view with one character, focus back to file1, then enter select mode
    // with a non-empty selection. After this the prompt-target is file1.
    let setup = format!(
        "iline1<ret>line2<ret>line3<esc><C-w>v:o {}<ret>ia<esc><C-w>w%v",
        file2.path().to_string_lossy()
    );
    send_keys(&tx, &setup)?;

    // Open the regex prompt (`s` = select_regex). This captures a snapshot of
    // the focused view+doc (file1, three lines).
    send_keys(&tx, "s")?;

    // Click into the right-hand view (file2). Mouse events pass through the
    // prompt; the editor's mouse handler calls `editor.focus(view_id)` and
    // shifts focus to file2 without aborting the prompt. Backend is 120
    // columns wide so the vertical split places file2 around column 80.
    let mouse = TerminalEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 90,
        row: 0,
        modifiers: TerminalKeyModifiers::empty(),
    });
    tx.send(Ok(mouse))?;

    // Abort the prompt. Pre-fix this restores the file1 snapshot into the
    // now-current file2 document and panics in `ensure_invariants`.
    send_keys(&tx, "<esc>")?;

    app.event_loop_until_idle(&mut rx_stream).await;

    // We never reach this point pre-fix; the panic above terminates the test
    // thread.
    helpers::assert_status_not_error(&app.editor);

    // file2 must still be a single-character document; the file1 snapshot
    // must not have leaked into it.
    let file2_normalized = helix_stdx::path::normalize(file2.path());
    let file2_doc = app
        .editor
        .documents()
        .find(|d| d.path().is_some_and(|p| p == file2_normalized))
        .expect("file2 should be open");
    assert_eq!(file2_doc.text().len_chars(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_changes_in_splits_jumplist_sync() -> anyhow::Result<()> {
    // See <https://github.com/helix-editor/helix/issues/9833>
    // When jumping backwards (<C-o>) switches between two documents, we need to
    // ensure that the current view has been synced with all changes to the
    // document that occurred since the last time the view focused this document.
    // If the view isn't synced then this case panics since we try to form a
    // selection on "test" (which was deleted in the other view).
    test((
        "#[test|]#",
        "<C-w>sgf<C-w>wd<C-w>w<C-o><C-w>qd",
        "#[|]#",
        LineFeedHandling::AsIs,
    ))
    .await?;

    Ok(())
}
