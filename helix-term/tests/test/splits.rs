use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_split_write_quit_all() -> anyhow::Result<()> {
    let mut file1 = tempfile::NamedTempFile::new()?;
    let mut file2 = tempfile::NamedTempFile::new()?;
    let mut file3 = tempfile::NamedTempFile::new()?;

    test_key_sequences(
        &mut helpers::app_with_file(file1.path())?,
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
                        .find(|doc| doc.path().unwrap() == file1.path())
                        .unwrap();

                    assert_eq!("hello1", doc1.text().to_string());

                    let doc2 = docs
                        .iter()
                        .find(|doc| doc.path().unwrap() == file2.path())
                        .unwrap();

                    assert_eq!("hello2", doc2.text().to_string());

                    let doc3 = docs
                        .iter()
                        .find(|doc| doc.path().unwrap() == file3.path())
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

    helpers::assert_file_has_content(file1.as_file_mut(), "hello1")?;
    helpers::assert_file_has_content(file2.as_file_mut(), "hello2")?;
    helpers::assert_file_has_content(file3.as_file_mut(), "hello3")?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_split_write_quit_same_file() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    test_key_sequences(
        &mut helpers::app_with_file(file.path())?,
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
                        helpers::platform_line("hello\ngoodbye"),
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
                        helpers::platform_line("hello\ngoodbye"),
                        doc.text().to_string()
                    );

                    assert!(!doc.is_modified());
                }),
            ),
        ],
        false,
    )
    .await?;

    helpers::assert_file_has_content(
        file.as_file_mut(),
        &helpers::platform_line("hello\ngoodbye"),
    )?;

    Ok(())
}
