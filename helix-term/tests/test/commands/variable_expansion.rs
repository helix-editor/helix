use super::*;
use helix_view::editor::expand_variables;

#[tokio::test(flavor = "multi_thread")]
async fn test_variable_expansion() -> anyhow::Result<()> {
    {
        let mut app = AppBuilder::new().build()?;

        assert_eq!(
            expand_variables(&app.editor, "%{filename}").unwrap(),
            helix_view::document::SCRATCH_BUFFER_NAME,
        );

        assert_eq!(
            expand_variables(&app.editor, "%{basename}").unwrap(),
            helix_view::document::SCRATCH_BUFFER_NAME,
        );

        assert_eq!(
            expand_variables(&app.editor, "%{dirname}").unwrap(),
            helix_view::document::SCRATCH_BUFFER_NAME,
        );
    }

    {
        let file = tempfile::NamedTempFile::new()?;
        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;

        assert_eq!(
            expand_variables(&app.editor, "%{filename}").unwrap(),
            std::fs::canonicalize(file.path().to_str().unwrap())
                .unwrap()
                .to_str()
                .unwrap()
        );

        assert_eq!(
            expand_variables(&app.editor, "%{basename}").unwrap(),
            file.path().file_name().unwrap().to_str().unwrap()
        );

        assert_eq!(
            expand_variables(&app.editor, "%{dirname}").unwrap(),
            std::fs::canonicalize(file.path().parent().unwrap().to_str().unwrap())
                .unwrap()
                .to_str()
                .unwrap()
        );
    }

    {
        let file = tempfile::NamedTempFile::new()?;
        let mut app = AppBuilder::new().with_file(file.path(), None).build()?;
        test_key_sequence(
            &mut app,
            Some("ihelix<esc>%"),
            Some(&|app| {
                assert_eq!(
                    expand_variables(&app.editor, "%{selection}").unwrap(),
                    "helix"
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
            Some("ihelix<ret>helix<ret>helix<ret><esc>"),
            Some(&|app| {
                assert_eq!(expand_variables(&app.editor, "%{linenumber}").unwrap(), "4");
            }),
            false,
        )
        .await?;
    }

    Ok(())
}
