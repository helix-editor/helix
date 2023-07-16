use std::{
    fs::File,
    io::{Read, Seek, Write},
    ops::RangeInclusive,
};

use helix_view::doc;

use crate::test::helpers::{
    assert_eq_contents,
    file::{assert_file_has_content, new_readonly_tempfile},
    test_harness::ActiveTestHarness,
};

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_write_scratch() -> anyhow::Result<()> {
    test_scratch(false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_quit_scratch() -> anyhow::Result<()> {
    test_scratch(true).await
}

async fn test_scratch(should_quit: bool) -> anyhow::Result<()> {
    let q = match should_quit {
        true => "q",
        false => "",
    };

    TestHarness::default()
        .push_test_case(
            TestCase::default()
                .with_keys(&format!("ihello<esc>:w{}<ret>", q))
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_document_count(1);
                    cx.assert_app_is_err();
                    assert!(doc!(cx.app.editor).path().is_none());
                })),
        )
        .run()
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_buffer_close_concurrent() -> anyhow::Result<()> {
    TestHarness::default()
        .push_test_case(TestCase::default().with_validation_fn(Box::new(|cx| {
            cx.assert_document_count(1);
            cx.assert_app_is_ok();
        })))
        .push_test_case(
            TestCase::default()
                .with_keys("ihello<esc>:new<ret>")
                .with_validation_fn(Box::new(|cx| {
                    cx.assert_document_count(2);
                    cx.assert_app_is_ok();
                })),
        )
        .push_test_case(
            TestCase::default()
                .with_keys(":buffer<minus>close<ret>")
                .with_validation_fn(Box::new(|cx| {
                    cx.assert_document_count(1);
                    cx.assert_app_is_ok();
                })),
        )
        .run()
        .await?;

    // verify if writes are queued up, it finishes them before closing the buffer
    let file = tempfile::NamedTempFile::new()?;
    let file_handle = File::open(file.path())?;
    const RANGE: RangeInclusive<i32> = 1..=1000;

    let mut command = String::new();
    for i in RANGE {
        command.push_str(&format!("%c{}<ret><esc>:w!<ret>", i));
    }
    command.push_str(":buffer<minus>close<ret>");

    TestHarness::default()
        .with_file(file.path())
        .push_test_case(
            TestCase::default()
                .with_keys(&command)
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_ok();
                    let doc = cx.app.editor.document_by_path(file.path());
                    assert!(doc.is_none(), "found doc: {:?}", doc);
                })),
        )
        .run()
        .await?;

    assert_eq_contents(file_handle, &RANGE.end().to_string());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let file_handle = File::open(file.path())?;
    const CONTENT: &str = "lorem ipsum";

    TestHarness::default()
        .with_file(file.path())
        .push_test_case(TestCase::default().with_keys(&format!("i{}<ret><esc>:w<ret>", CONTENT)))
        .run()
        .await?;

    assert_eq_contents(file_handle, CONTENT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_overwrite_protection() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    let mut active_test_harness: ActiveTestHarness = TestHarness::default()
        .with_file(file.path())
        .push_test_case(TestCase::default().with_keys(":x<ret>"))
        .into();

    active_test_harness.app.tick().await;

    const CONTENT: &str = "extremely important content";

    file.write_all(helpers::platform_line(CONTENT).as_bytes())?;

    active_test_harness.finish().await?;

    file.rewind()?;
    assert_eq_contents(file.reopen().unwrap(), CONTENT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_quit() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let file_handle = File::open(file.path())?;
    const CONTENT: &str = "lorem ipsum";

    TestHarness::default()
        .with_file(file.path())
        .should_exit()
        .push_test_case(TestCase::default().with_keys(&format!("i{}<ret><esc>:wq<ret>", CONTENT)))
        .run()
        .await?;

    assert_eq_contents(file_handle, CONTENT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_concurrent() -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    const RANGE: RangeInclusive<i32> = 1..=1000;

    let mut command = String::new();
    for i in RANGE {
        command.push_str(&format!("%c{}<esc>:w!<ret>", i));
    }

    TestHarness::default()
        .with_file(file.path())
        .push_test_case(TestCase::default().with_keys(&command))
        .run()
        .await?;

    file.as_file_mut().flush()?;
    file.as_file_mut().sync_all()?;

    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;
    assert_eq!(RANGE.end().to_string(), file_content);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_fail_mod_flag() -> anyhow::Result<()> {
    let file = new_readonly_tempfile()?;

    TestHarness::default()
        .with_file(file.path())
        .push_test_case(
            TestCase::default()
                .with_validation_fn(Box::new(|cx| assert!(!cx.newest_doc_is_modified()))),
        )
        .push_test_case(
            TestCase::default()
                .with_keys("ihello<esc>")
                .with_validation_fn(Box::new(|cx| {
                    cx.assert_app_is_ok();
                    assert!(cx.newest_doc_is_modified())
                })),
        )
        .push_test_case(
            TestCase::default()
                .with_keys(":w<ret>")
                .with_validation_fn(Box::new(|cx| {
                    cx.assert_app_is_err();
                    assert!(cx.newest_doc_is_modified())
                })),
        )
        .run()
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_scratch_to_new_path() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let file_handle = File::open(file.path())?;
    const CONTENT: &str = "hello";

    TestHarness::default()
        .with_file(file.path())
        .push_test_case(
            TestCase::default()
                .with_keys(
                    format!(
                        "i{}<ret><esc>:w {}<ret>",
                        CONTENT,
                        file.path().to_string_lossy()
                    )
                    .as_ref(),
                )
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_ok();
                    cx.assert_document_count(1);
                    cx.assert_eq_document_path(file.path())
                })),
        )
        .run()
        .await?;

    assert_eq_contents(file_handle, CONTENT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_auto_format_fails_still_writes() -> anyhow::Result<()> {
    let mut file = tempfile::Builder::new().suffix(".rs").tempfile()?;

    let lang_conf = indoc::indoc! {r#"
            [[language]]
            name = "rust"
            formatter = { command = "bash", args = [ "-c", "exit 1" ] }
        "#};

    let app_config = helpers::AppBuilder::default()
        .with_file(file.path())
        .lang_config_overrides(lang_conf.into());

    test!(
        app_config,
        ("#[l|]#et foo = 0;"),
        (":w<ret>"),
        ("#[l|]#et foo = 0;")
    )
    .await?;

    assert_file_has_content(file.as_file_mut(), "let foo = 0;\n")
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_new_path() -> anyhow::Result<()> {
    let file1 = tempfile::NamedTempFile::new().unwrap();
    let file2 = tempfile::NamedTempFile::new().unwrap();

    let file_handle1 = File::open(file1.path()).unwrap();
    let file_handle2 = File::open(file2.path()).unwrap();

    const CONTENT: &str = "i can eat glass, it will not hurt me";

    TestHarness::default()
        .with_file(file1.path())
        .push_test_case(
            TestCase::default()
                .with_keys(&format!("i{}<ret><esc>:w<ret>", CONTENT))
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_ok();
                    cx.assert_eq_document_path(file1.path());
                })),
        )
        .push_test_case(
            TestCase::default()
                .with_keys(&format!(":w {}<ret>", file2.path().to_string_lossy()))
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_ok();
                    cx.assert_eq_document_path(file2.path());
                })),
        )
        .run()
        .await?;

    assert_eq_contents(file_handle1, CONTENT);
    assert_eq_contents(file_handle2, CONTENT);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_write_fail_new_path() -> anyhow::Result<()> {
    let file = new_readonly_tempfile()?;

    TestHarness::default()
        .push_test_case(TestCase::default().with_validation_fn(Box::new(move |cx| {
            cx.assert_app_is_ok();
            assert!(doc!(cx.app.editor).path().is_none())
        })))
        .push_test_case(
            TestCase::default()
                .with_keys(&format!(":w {}<ret>", file.path().to_string_lossy()))
                .with_validation_fn(Box::new(move |cx| {
                    cx.assert_app_is_err();
                    assert!(doc!(cx.app.editor).path().is_none())
                })),
        )
        .run()
        .await
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
    edit_file_with_content(&UTF16BE_FILE).await
}

async fn edit_file_with_content(file_content: &[u8]) -> anyhow::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;

    file.as_file_mut().write_all(file_content)?;

    TestHarness::default()
        .should_exit()
        .push_test_case(
            TestCase::default()
                .with_keys(&format!(":o {}<ret>:x<ret>", file.path().to_string_lossy())),
        )
        .run()
        .await?;

    file.rewind()?;
    let mut new_file_content: Vec<u8> = Vec::new();
    file.read_to_end(&mut new_file_content)?;

    assert_eq!(file_content, new_file_content);

    Ok(())
}
