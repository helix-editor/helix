use std::{
    io::{Read, Seek, Write},
    ops::RangeInclusive,
};

use helix_core::{diagnostic::Severity, path::get_normalized_path};
use helix_view::doc;

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
            assert_eq!(Some(&get_normalized_path(file.path())), doc.path());
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
        let cmd = format!("%c{}<esc>:w!<ret>", i);
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
async fn test_write() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    test_key_sequence(
        &mut app,
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
async fn test_overwrite_protection() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    helpers::run_event_loop_until_idle(&mut app).await;

    file.as_file_mut()
        .write_all(helpers::platform_line("extremely important content").as_bytes())?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    test_key_sequence(&mut app, Some(":x<ret>"), None, false).await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    file.rewind()?;
    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;

    assert_eq!(
        helpers::platform_line("extremely important content"),
        file_content
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_quit() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    test_key_sequence(
        &mut app,
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

#[tokio::test(flavor = "multi_thread")]
async fn test_write_concurrent() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    let mut command = String::new();
    const RANGE: RangeInclusive<i32> = 1..=1000;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    for i in RANGE {
        let cmd = format!("%c{}<esc>:w!<ret>", i);
        command.push_str(&cmd);
    }

    test_key_sequence(&mut app, Some(&command), None, false).await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;
    assert_eq!(RANGE.end().to_string(), file_content);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_fail_mod_flag() -> anyhow::Result<()> {
    let file = helpers::new_readonly_tempfile()?;
    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .build()?;

    test_key_sequences(
        &mut app,
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

#[tokio::test(flavor = "multi_thread")]
async fn test_write_scratch_to_new_path() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(format!("ihello<esc>:w {}<ret>", file.path().to_string_lossy()).as_ref()),
        Some(&|app| {
            assert!(!app.editor.is_err());

            let mut docs: Vec<_> = app.editor.documents().collect();
            assert_eq!(1, docs.len());

            let doc = docs.pop().unwrap();
            assert_eq!(Some(&get_normalized_path(file.path())), doc.path());
        }),
        false,
    )
    .await?;

    helpers::assert_file_has_content(file.as_file_mut(), &helpers::platform_line("hello"))?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_scratch_no_path_fails() -> anyhow::Result<()> {
    helpers::test_key_sequence_with_input_text(
        None,
        ("#[\n|]#", "ihello<esc>:w<ret>", "hello#[\n|]#"),
        &|app| {
            assert!(app.editor.is_err());

            let mut docs: Vec<_> = app.editor.documents().collect();
            assert_eq!(1, docs.len());

            let doc = docs.pop().unwrap();
            assert_eq!(None, doc.path());
        },
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_auto_format_fails_still_writes() -> anyhow::Result<()> {
    let mut file = tempfile::Builder::new().suffix(".rs").tempfile()?;

    let lang_conf = indoc! {r#"
            [[language]]
            name = "rust"
            formatter = { command = "bash", args = [ "-c", "exit 1" ] }
        "#};

    let mut app = helpers::AppBuilder::new()
        .with_file(file.path(), None)
        .with_input_text("#[l|]#et foo = 0;\n")
        .with_lang_config(helpers::test_syntax_conf(Some(lang_conf.into())))
        .build()?;

    test_key_sequences(&mut app, vec![(Some(":w<ret>"), None)], false).await?;

    // file still saves
    helpers::assert_file_has_content(file.as_file_mut(), "let foo = 0;\n")?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_new_path() -> anyhow::Result<()> {
    let mut file1 = tempfile::NamedTempFile::new().unwrap();
    let mut file2 = tempfile::NamedTempFile::new().unwrap();
    let mut app = helpers::AppBuilder::new()
        .with_file(file1.path(), None)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some("ii can eat glass, it will not hurt me<ret><esc>:w<ret>"),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert!(!app.editor.is_err());
                    assert_eq!(&get_normalized_path(file1.path()), doc.path().unwrap());
                }),
            ),
            (
                Some(&format!(":w {}<ret>", file2.path().to_string_lossy())),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert!(!app.editor.is_err());
                    assert_eq!(&get_normalized_path(file2.path()), doc.path().unwrap());
                    assert!(app.editor.document_by_path(file1.path()).is_none());
                }),
            ),
        ],
        false,
    )
    .await?;

    helpers::assert_file_has_content(
        file1.as_file_mut(),
        &helpers::platform_line("i can eat glass, it will not hurt me\n"),
    )?;

    helpers::assert_file_has_content(
        file2.as_file_mut(),
        &helpers::platform_line("i can eat glass, it will not hurt me\n"),
    )?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_fail_new_path() -> anyhow::Result<()> {
    let file = helpers::new_readonly_tempfile()?;

    test_key_sequences(
        &mut AppBuilder::new().build()?,
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

#[tokio::test(flavor = "multi_thread")]
async fn test_write_utf_bom_file() -> anyhow::Result<()> {
    // "ABC" with utf8 bom
    const UTF8_FILE: [u8; 6] = [0xef, 0xbb, 0xbf, b'A', b'B', b'C'];

    // "ABC" in UTF16 with bom
    const UTF16LE_FILE: [u8; 8] = [0xff, 0xfe, b'A', 0x00, b'B', 0x00, b'C', 0x00];
    const UTF16BE_FILE: [u8; 8] = [0xfe, 0xff, 0x00, b'A', 0x00, b'B', 0x00, b'C'];

    edit_file_with_content(&UTF8_FILE).await?;
    edit_file_with_content(&UTF16LE_FILE).await?;
    edit_file_with_content(&UTF16BE_FILE).await?;

    Ok(())
}

async fn edit_file_with_content(file_content: &[u8]) -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    file.as_file_mut().write_all(&file_content)?;

    helpers::test_key_sequence(
        &mut helpers::AppBuilder::new().build()?,
        Some(&format!(":o {}<ret>:x<ret>", file.path().to_string_lossy())),
        None,
        true,
    )
    .await?;

    file.rewind()?;
    let mut new_file_content: Vec<u8> = Vec::new();
    file.read_to_end(&mut new_file_content)?;

    assert_eq!(file_content, new_file_content);

    Ok(())
}
