use std::{
    io::{Read, Write},
    ops::RangeInclusive,
};

use helix_core::diagnostic::Severity;
use helix_term::application::Application;
use helix_view::doc;

use super::*;

#[tokio::test]
async fn test_write() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    test_key_sequence(
        &mut helpers::app_with_file(file.path())?,
        Some("ithe gostak distims the doshes<ret><esc>:w<ret>"),
        None,
        false,
    )
    .await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;

    assert_eq!(
        helpers::platform_line("the gostak distims the doshes"),
        file_content
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_quit() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    test_key_sequence(
        &mut helpers::app_with_file(file.path())?,
        Some("ithe gostak distims the doshes<ret><esc>:wq<ret>"),
        None,
        true,
    )
    .await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;

    assert_eq!(
        helpers::platform_line("the gostak distims the doshes"),
        file_content
    );

    Ok(())
}

#[tokio::test]
async fn test_write_concurrent() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    let mut command = String::new();
    const RANGE: RangeInclusive<i32> = 1..=5000;

    for i in RANGE {
        let cmd = format!("%c{}<esc>:w<ret>", i);
        command.push_str(&cmd);
    }

    test_key_sequence(
        &mut helpers::app_with_file(file.path())?,
        Some(&command),
        None,
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
async fn test_write_fail_mod_flag() -> anyhow::Result<()> {
    let file = helpers::new_readonly_tempfile()?;

    test_key_sequences(
        &mut helpers::app_with_file(file.path())?,
        vec![
            (
                None,
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert!(!doc.is_modified());
                }),
            ),
            (
                Some("ihello<esc>"),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert!(doc.is_modified());
                }),
            ),
            (
                Some(":w<ret>"),
                Some(&|app| {
                    assert_eq!(&Severity::Error, app.editor.get_status().unwrap().1);

                    let doc = doc!(app.editor);
                    assert!(doc.is_modified());
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_write_new_path() -> anyhow::Result<()> {
    let mut file1 = tempfile::NamedTempFile::new().unwrap();
    let mut file2 = tempfile::NamedTempFile::new().unwrap();

    test_key_sequences(
        &mut Application::new(
            Args {
                files: vec![(file1.path().to_path_buf(), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )?,
        vec![
            (
                Some("ii can eat glass, it will not hurt me<ret><esc>:w<ret>"),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert!(!app.editor.is_err());
                    assert_eq!(file1.path(), doc.path().unwrap());
                }),
            ),
            (
                Some(&format!(":w {}<ret>", file2.path().to_string_lossy())),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert!(!app.editor.is_err());
                    assert_eq!(file2.path(), doc.path().unwrap());
                    assert!(app.editor.document_by_path(file1.path()).is_none());
                }),
            ),
        ],
        false,
    )
    .await?;

    file1.as_file_mut().flush()?;
    file1.as_file_mut().sync_all()?;
    file2.as_file_mut().flush()?;
    file2.as_file_mut().sync_all()?;

    let mut file1_content = String::new();
    file1.as_file_mut().read_to_string(&mut file1_content)?;
    assert_eq!(
        helpers::platform_line("i can eat glass, it will not hurt me\n"),
        file1_content
    );

    let mut file2_content = String::new();
    file2.as_file_mut().read_to_string(&mut file2_content)?;
    assert_eq!(
        helpers::platform_line("i can eat glass, it will not hurt me\n"),
        file2_content
    );

    Ok(())
}

#[tokio::test]
async fn test_write_fail_new_path() -> anyhow::Result<()> {
    let file = helpers::new_readonly_tempfile()?;

    test_key_sequences(
        &mut Application::new(Args::default(), Config::default())?,
        vec![
            (
                None,
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert_ne!(
                        Some(&Severity::Error),
                        app.editor.get_status().map(|status| status.1)
                    );
                    assert_eq!(None, doc.path());
                }),
            ),
            (
                Some(&format!(":w {}<ret>", file.path().to_string_lossy())),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert_eq!(
                        Some(&Severity::Error),
                        app.editor.get_status().map(|status| status.1)
                    );
                    assert_eq!(None, doc.path());
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}
