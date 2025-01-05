use super::*;
use std::borrow::Cow;
#[tokio::test(flavor = "multi_thread")]
async fn test_variable_expansion() -> anyhow::Result<()> {
    {
        let mut app = AppBuilder::new().build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %{filename}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    helix_view::document::SCRATCH_BUFFER_NAME
                );
            }),
            false,
        )
        .await?;
        let mut app = AppBuilder::new().build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %{basename}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    helix_view::document::SCRATCH_BUFFER_NAME
                );
            }),
            false,
        )
        .await?;

        let mut app = AppBuilder::new().build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %{dirname}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    helix_view::document::SCRATCH_BUFFER_NAME
                );
            }),
            false,
        )
        .await?;
    }

    {
        let file = tempfile::NamedTempFile::new()?;
        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %{filename}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    helix_stdx::path::canonicalize(file.path())
                        .to_str()
                        .unwrap()
                );
            }),
            false,
        )
        .await?;

        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %{basename}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    file.path().file_name().unwrap().to_str().unwrap()
                );
            }),
            false,
        )
        .await?;

        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %{dirname}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    helix_stdx::path::canonicalize(file.path().parent().unwrap())
                        .to_str()
                        .unwrap()
                );
            }),
            false,
        )
        .await?;
    }

    {
        let file = tempfile::NamedTempFile::new()?;
        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;
        test_key_sequence(
            &mut app,
            Some("ihelix<esc>%:echo %{selection}<ret>"),
            Some(&|app| {
                assert_eq!(app.editor.get_status().unwrap().0, "helix");
            }),
            false,
        )
        .await?;
    }

    {
        let file = tempfile::NamedTempFile::new()?;
        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;
        test_key_sequence(
            &mut app,
            Some("ihelix<ret>helix<ret>helix<ret><esc>:echo %{linenumber}<ret>"),
            Some(&|app| {
                assert_eq!(app.editor.get_status().unwrap().0, "4");
            }),
            false,
        )
        .await?;

        let mut app = AppBuilder::new().build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %sh{echo %{filename}}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    helix_view::document::SCRATCH_BUFFER_NAME
                );
            }),
            false,
        )
        .await?;
        let mut app = AppBuilder::new().build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %sh{echo %{filename} %{linenumber}}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    &Cow::from(format!(
                        "{} {}",
                        helix_view::document::SCRATCH_BUFFER_NAME,
                        1
                    ))
                );
            }),
            false,
        )
        .await?;
        let mut app = AppBuilder::new().build()?;

        test_key_sequence(
            &mut app,
            Some("<esc>:echo %sh{echo %{filename} %sh{echo %{filename}}}<ret>"),
            Some(&|app| {
                assert_eq!(
                    app.editor.get_status().unwrap().0,
                    &Cow::from(format!(
                        "{} {}",
                        helix_view::document::SCRATCH_BUFFER_NAME,
                        helix_view::document::SCRATCH_BUFFER_NAME
                    ))
                );
            }),
            false,
        )
        .await?;
    }

    Ok(())
}
