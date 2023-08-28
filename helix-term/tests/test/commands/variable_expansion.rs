use super::*;
use helix_view::editor::expand_variables;

#[tokio::test(flavor = "multi_thread")]
async fn test_variable_expansion() -> anyhow::Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    let mut app = AppBuilder::new().with_file(file.path(), None).build()?;

    assert_eq!(
        expand_variables(&app.editor, "%{filename}").unwrap(),
        file.path().to_str().unwrap()
    );

    assert_eq!(
        expand_variables(&app.editor, "%{basename}").unwrap(),
        file.path().file_name().unwrap().to_str().unwrap()
    );

    assert_eq!(
        expand_variables(&app.editor, "%{dirname}").unwrap(),
        file.path().parent().unwrap().to_str().unwrap()
    );

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
            Some("ihelix<esc>%"),
            Some(&|app| {
                assert_eq!(expand_variables(&app.editor, "%{linenumber}").unwrap(), "1");
            }),
            false,
        )
        .await?;
    }

    Ok(())
}
