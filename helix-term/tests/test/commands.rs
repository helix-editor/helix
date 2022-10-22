use std::ops::RangeInclusive;

use helix_core::diagnostic::Severity;

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_write_quit_fail() -> anyhow::Result<()> {
    let file = helpers::new_readonly_tempfile()?;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    test_key_sequence(
        &mut app,
        Some("ihello<esc>:wq<ret>"),
        Some(&|app| {
            let mut docs: Vec<_> = app.editor.documents().collect();
            assert_eq!(1, docs.len());

            let doc = docs.pop().unwrap();
            assert_eq!(Some(file.path()), doc.path().map(PathBuf::as_path));
            assert_eq!(&Severity::Error, app.editor.get_status().unwrap().1);
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_buffer_close_concurrent() -> anyhow::Result<()> {
    test_key_sequences(
        &mut helpers::AppBuilder::new().build()?,
        vec![
            (
                None,
                Some(&|app| {
                    assert_eq!(1, app.editor.documents().count());
                    assert!(!app.editor.is_err());
                }),
            ),
            (
                Some("ihello<esc>:new<ret>"),
                Some(&|app| {
                    assert_eq!(2, app.editor.documents().count());
                    assert!(!app.editor.is_err());
                }),
            ),
            (
                Some(":buffer<minus>close<ret>"),
                Some(&|app| {
                    assert_eq!(1, app.editor.documents().count());
                    assert!(!app.editor.is_err());
                }),
            ),
        ],
        false,
    )
    .await?;

    // verify if writes are queued up, it finishes them before closing the buffer
    let mut file = tempfile::NamedTempFile::new()?;
    let mut command = String::new();
    const RANGE: RangeInclusive<i32> = 1..=1000;

    for i in RANGE {
        let cmd = format!("%c{}<esc>:w<ret>", i);
        command.push_str(&cmd);
    }

    command.push_str(":buffer<minus>close<ret>");

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    test_key_sequence(
        &mut app,
        Some(&command),
        Some(&|app| {
            assert!(!app.editor.is_err(), "error: {:?}", app.editor.get_status());

            let doc = app.editor.document_by_path(file.path());
            assert!(doc.is_none(), "found doc: {:?}", doc);
        }),
        false,
    )
    .await?;

    helpers::assert_file_has_content(file.as_file_mut(), &RANGE.end().to_string())?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_selection_duplication() -> anyhow::Result<()> {
    // Forward
    test((
        platform_line(indoc! {"\
            #[lo|]#rem
            ipsum
            dolor
            "})
        .as_str(),
        "CC",
        platform_line(indoc! {"\
            #(lo|)#rem
            #(ip|)#sum
            #[do|]#lor
            "})
        .as_str(),
    ))
    .await?;

    // Backward
    test((
        platform_line(indoc! {"\
            #[|lo]#rem
            ipsum
            dolor
            "})
        .as_str(),
        "CC",
        platform_line(indoc! {"\
            #(|lo)#rem
            #(|ip)#sum
            #[|do]#lor
            "})
        .as_str(),
    ))
    .await?;
    Ok(())
}
