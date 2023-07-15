use crate::test::helpers::{
    file::assert_file_has_content,
    platform_line,
    test_harness::{test, TestCase, TestHarness},
};

#[tokio::test(flavor = "multi_thread")]
async fn test_split_write_quit_all() -> anyhow::Result<()> {
    let mut file1 = tempfile::NamedTempFile::new()?;
    let mut file2 = tempfile::NamedTempFile::new()?;
    let mut file3 = tempfile::NamedTempFile::new()?;

    TestHarness::default()
        .with_file(file1.path())
        .should_exit()
        .push_test_case(
            TestCase::default()
                .with_keys(&format!(
                    "ihello1<esc>:sp<ret>:o {}<ret>ihello2<esc>:sp<ret>:o {}<ret>ihello3<esc>",
                    file2.path().to_string_lossy(),
                    file3.path().to_string_lossy()
                ))
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_ok();
                    cx.assert_view_count(3);
                    cx.assert_document_count(3);

                    let mut doc_texts = cx
                        .app
                        .editor
                        .documents
                        .values()
                        .map(|document| document.text());

                    assert_eq!("hello1", doc_texts.next().unwrap());
                    assert_eq!("hello2", doc_texts.next().unwrap());
                    assert_eq!("hello3", doc_texts.next().unwrap());
                })),
        )
        .push_test_case(
            TestCase::default()
                .with_keys(":wqa<ret>")
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_ok();
                    cx.assert_view_count(0);
                })),
        )
        .run()
        .await?;

    assert_file_has_content(file1.as_file_mut(), "hello1")?;
    assert_file_has_content(file2.as_file_mut(), "hello2")?;
    assert_file_has_content(file3.as_file_mut(), "hello3")
}

#[tokio::test(flavor = "multi_thread")]
async fn test_split_write_quit_same_file() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    TestHarness::default()
        .with_file(file.path())
        .push_test_case(split_test_case(
            "O<esc>ihello<esc>:sp<ret>ogoodbye<esc>",
            2,
            true,
        ))
        .push_test_case(split_test_case(":wq<ret>", 1, false))
        .run()
        .await?;

    return assert_file_has_content(file.as_file_mut(), &platform_line("hello\ngoodbye"));

    fn split_test_case(key_str: &str, view_count: usize, doc_is_modified: bool) -> TestCase {
        TestCase::default()
            .with_keys(key_str)
            .with_expected_text("hello\ngoodbye")
            .with_validation_fn(Box::new(move |cx| {
                cx.assert_app_is_ok();
                cx.assert_view_count(view_count);
                cx.assert_document_count(1);
                cx.assert_eq_text_current();
                assert!(cx.newest_doc_is_modified() == doc_is_modified)
            }))
    }
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
    test(("#[|]#", "<C-w>v[<space><C-s><C-w>wkd<C-w>qd", "#[|]#")).await?;

    // Transactions are applied to the views for windows lazily when they are focused.
    // This case panics if the transactions and inversions are not applied in the
    // correct order as we switch between windows.
    test((
        "#[|]#",
        "[<space>[<space>[<space><C-w>vuuu<C-w>wUUU<C-w>quuu",
        "#[|]#",
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
    ))
    .await
}
