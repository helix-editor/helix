use std::fs;

use helix_core::{path::get_canonicalized_path, Range};
use helix_loader::{current_working_dir, set_current_working_dir};
use helix_view::{current_ref, editor::Action};
use tempfile::{Builder, TempDir};

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_picker_alt_ret() -> anyhow::Result<()> {
    // Create two files, open the first and run a global search for a word
    // from the second file. Press <alt-ret> to have helix open the second file in the
    // new buffer, but not change focus. Then check whether the word is highlighted
    // correctly and the view of the first file has not changed.
    let tmp_dir = TempDir::new()?;
    set_current_working_dir(tmp_dir.path().into())?;

    let mut app = AppBuilder::new().build()?;

    log::debug!(
        "set current working directory to {:?}",
        current_working_dir()
    );

    // Add prefix so helix doesn't hide these files in a picker
    let files = [
        Builder::new().prefix("1").tempfile_in(&tmp_dir)?,
        Builder::new().prefix("2").tempfile_in(&tmp_dir)?,
    ];
    let paths = files
        .iter()
        .map(|f| get_canonicalized_path(f.path()))
        .collect::<Vec<_>>();

    fs::write(&paths[0], "1\n2\n3\n4")?;
    fs::write(&paths[1], "first\nsecond")?;

    log::debug!(
        "created and wrote two temporary files: {:?} & {:?}",
        paths[0],
        paths[1]
    );

    // Manually open to save the offset, otherwise we won't be able to change the state in the Fn trait
    app.editor.open(files[0].path(), Action::Replace)?;
    let view_offset = current_ref!(app.editor).0.offset;

    test_key_sequences(
        &mut app,
        vec![
            (Some("<space>/"), None),
            (Some("second<ret>"), None),
            (
                Some("<A-ret><esc>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(app.editor);
                    assert_eq!(doc.path().unwrap(), &paths[0]);
                    let select_ranges = doc.selection(view.id).ranges();
                    assert_eq!(select_ranges[0], Range::new(0, 1));
                    assert_eq!(view.offset, view_offset);
                }),
            ),
            (
                Some(":buffer<minus>next<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(app.editor);
                    assert_eq!(doc.path().unwrap(), &paths[1]);
                    let select_ranges = doc.selection(view.id).ranges();
                    assert_eq!(select_ranges.len(), 1);
                    assert_eq!(select_ranges[0], Range::new(6, 12));
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}
