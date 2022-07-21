use std::{
    io::{Read, Write},
    ops::RangeInclusive,
};

use helix_core::diagnostic::Severity;
use helix_term::application::Application;

use super::*;

#[tokio::test]
async fn test_write_quit_fail() -> anyhow::Result<()> {
    let file = helpers::new_readonly_tempfile()?;

    test_key_sequence(
        &mut helpers::app_with_file(file.path())?,
        Some("ihello<esc>:wq<ret>"),
        Some(&|app| {
            assert_eq!(&Severity::Error, app.editor.get_status().unwrap().1);
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_buffer_close_concurrent() -> anyhow::Result<()> {
    test_key_sequences(
        &mut Application::new(Args::default(), Config::default())?,
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

    test_key_sequence(
        &mut helpers::app_with_file(file.path())?,
        Some(&command),
        Some(&|app| {
            assert!(!app.editor.is_err(), "error: {:?}", app.editor.get_status());

            let doc = app.editor.document_by_path(file.path());
            assert!(doc.is_none(), "found doc: {:?}", doc);
        }),
        false,
    )
    .await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;
    assert_eq!(RANGE.end().to_string(), file_content);

    Ok(())
}

#[tokio::test]
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
