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
        &mut Application::new(
            Args {
                files: vec![(file.path().to_path_buf(), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )?,
        Some("ii can eat glass, it will not hurt me<ret><esc>:w<ret>"),
        None,
    )
    .await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;
    assert_eq!("i can eat glass, it will not hurt me\n", file_content);

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
        &mut Application::new(
            Args {
                files: vec![(file.path().to_path_buf(), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )?,
        Some(&command),
        None,
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
    test_key_sequences(
        &mut Application::new(
            Args {
                files: vec![(PathBuf::from("/foo"), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )?,
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
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_write_fail_new_path() -> anyhow::Result<()> {
    test_key_sequences(
        &mut Application::new(Args::default(), Config::default())?,
        vec![
            (
                None,
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert_eq!(None, app.editor.get_status());
                    assert_eq!(None, doc.path());
                }),
            ),
            (
                Some(":w /foo<ret>"),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert_eq!(&Severity::Error, app.editor.get_status().unwrap().1);
                    assert_eq!(None, doc.path());
                }),
            ),
        ],
    )
    .await?;

    Ok(())
}
