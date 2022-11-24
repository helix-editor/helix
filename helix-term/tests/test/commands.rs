use std::ops::RangeInclusive;

use helix_core::diagnostic::Severity;
use helix_term::application::Application;

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

#[tokio::test(flavor = "multi_thread")]
async fn test_goto_file_impl() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::new()?;

    fn match_paths(app: &Application, matches: Vec<&str>) -> usize {
        app.editor
            .documents()
            .filter_map(|d| d.path()?.file_name())
            .filter(|n| matches.iter().any(|m| *m == n.to_string_lossy()))
            .count()
    }

    // Single selection
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("ione.js<esc>%gf"),
        Some(&|app| {
            assert_eq!(1, match_paths(app, vec!["one.js"]));
        }),
        false,
    )
    .await?;

    // Multiple selection
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("ione.js<ret>two.js<esc>%<A-s>gf"),
        Some(&|app| {
            assert_eq!(2, match_paths(app, vec!["one.js", "two.js"]));
        }),
        false,
    )
    .await?;

    // Cursor on first quote
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("iimport 'one.js'<esc>B;gf"),
        Some(&|app| {
            assert_eq!(1, match_paths(app, vec!["one.js"]));
        }),
        false,
    )
    .await?;

    // Cursor on last quote
    test_key_sequence(
        &mut AppBuilder::new().with_file(file.path(), None).build()?,
        Some("iimport 'one.js'<esc>bgf"),
        Some(&|app| {
            assert_eq!(1, match_paths(app, vec!["one.js"]));
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_selection_paste() -> anyhow::Result<()> {
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "})
        .as_str(),
        "yp",
        platform_line(indoc! {"\
            lorem#[|lorem]#
            ipsum#(|ipsum)#
            dolor#(|dolor)#
            "})
        .as_str(),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_selection_shell_commands() -> anyhow::Result<()> {
    // pipe
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "})
        .as_str(),
        "|echo foo<ret>",
        platform_line(indoc! {"\
            #[|foo
            ]#
            #(|foo
            )#
            #(|foo
            )#
            "})
        .as_str(),
    ))
    .await?;

    // insert-output
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "})
        .as_str(),
        "!echo foo<ret>",
        platform_line(indoc! {"\
            #[|foo
            ]#lorem
            #(|foo
            )#ipsum
            #(|foo
            )#dolor
            "})
        .as_str(),
    ))
    .await?;

    // append-output
    test((
        platform_line(indoc! {"\
            #[|lorem]#
            #(|ipsum)#
            #(|dolor)#
            "})
        .as_str(),
        "<A-!>echo foo<ret>",
        platform_line(indoc! {"\
            lorem#[|foo
            ]#
            ipsum#(|foo
            )#
            dolor#(|foo
            )#
            "})
        .as_str(),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_undo_redo() -> anyhow::Result<()> {
    // A jumplist selection is created at a point which is undone.
    //
    // * 2[<space>   Add two newlines at line start. We're now on line 3.
    // * <C-s>       Save the selection on line 3 in the jumplist.
    // * u           Undo the two newlines. We're now on line 1.
    // * <C-o><C-i>  Jump forward an back again in the jumplist. This would panic
    //               if the jumplist were not being updated correctly.
    test(("#[|]#", "2[<space><C-s>u<C-o><C-i>", "#[|]#")).await?;

    // A jumplist selection is passed through an edit and then an undo and then a redo.
    //
    // * [<space>    Add a newline at line start. We're now on line 2.
    // * <C-s>       Save the selection on line 2 in the jumplist.
    // * kd          Delete line 1. The jumplist selection should be adjusted to the new line 1.
    // * uU          Undo and redo the `kd` edit.
    // * <C-o>       Jump back in the jumplist. This would panic if the jumplist were not being
    //               updated correctly.
    // * <C-i>       Jump forward to line 1.
    test(("#[|]#", "[<space><C-s>kduU<C-o><C-i>", "#[|]#")).await?;

    // In this case we 'redo' manually to ensure that the transactions are composing correctly.
    test(("#[|]#", "[<space>u[<space>u", "#[|]#")).await?;

    Ok(())
}
