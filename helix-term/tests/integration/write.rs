use std::{
    io::{Read, Write},
    ops::RangeInclusive,
};

use helix_term::application::Application;

use super::*;

#[tokio::test]
async fn test_write() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new().unwrap();

    test_key_sequence(
        &mut Application::new(
            Args {
                files: vec![(file.path().to_path_buf(), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )?,
        "ii can eat glass, it will not hurt me<ret><esc>:w<ret>",
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
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let mut command = String::new();
    const RANGE: RangeInclusive<i32> = 1..=1000;

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
        &command,
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
