use std::{fs, io::Write, time::Duration};

use helix_core::diagnostic::Severity;
use helix_term::application::Application;
use helix_view::doc;

use super::*;

async fn wait_until(
    app: &mut Application,
    predicate: impl Fn(&Application) -> bool,
) -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate(app) {
            let event = app.editor.wait_event().await;
            app.handle_editor_event(event).await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn reloads_clean_buffer_after_external_change() -> anyhow::Result<()> {
    let file = temp_file_with_contents("original\n")?;
    let mut app = AppBuilder::new().with_file(file.path(), None).build()?;

    fs::write(file.path(), "externally updated\n")?;

    wait_until(&mut app, |app| {
        *doc!(app.editor).text() == "externally updated\n"
    })
    .await?;

    assert!(!doc!(app.editor).is_modified());
    assert!(app.editor.get_status().is_some_and(|(message, severity)| {
        *severity == Severity::Info && message.contains("reloaded because it changed on disk")
    }));

    assert!(app.close().await.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn preserves_modified_buffer_after_external_change() -> anyhow::Result<()> {
    let file = temp_file_with_contents("original\n")?;
    let mut app = AppBuilder::new()
        .with_file(file.path(), None)
        .with_input_text("#[unsaved buffer|]#\n")
        .build()?;

    fs::write(file.path(), "externally updated with different length\n")?;

    wait_until(&mut app, |app| {
        app.editor.get_status().is_some_and(|(message, severity)| {
            *severity == Severity::Warning
                && message.contains("changed on disk; buffer has unsaved changes")
        })
    })
    .await?;

    assert_eq!(doc!(app.editor).text().to_string(), "unsaved buffer\n");
    assert!(doc!(app.editor).is_modified());

    assert!(app.close().await.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn does_not_reload_after_helix_writes_file() -> anyhow::Result<()> {
    let file = temp_file_with_contents("original\n")?;
    let mut app = AppBuilder::new()
        .with_file(file.path(), None)
        .with_input_text("#[written by helix|]#\n")
        .build()?;
    let (view, document) = helix_view::current!(app.editor);
    document.append_changes_to_history(view);
    let doc_id = doc!(app.editor).id();

    app.editor.save(doc_id, None::<std::path::PathBuf>, false)?;
    while app.editor.write_count != 0 {
        let event = app.editor.wait_event().await;
        app.handle_editor_event(event).await;
    }

    let settle = tokio::time::sleep(Duration::from_millis(300));
    tokio::pin!(settle);
    loop {
        tokio::select! {
            _ = &mut settle => break,
            event = app.editor.wait_event() => app.handle_editor_event(event).await,
        };
    }

    assert!(app.editor.get_status().is_some_and(|(message, severity)| {
        *severity == Severity::Info && message.contains("written")
    }));
    assert!(!doc!(app.editor).is_modified());

    assert!(app.close().await.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn reloads_after_atomic_file_replacement() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("watched.txt");
    fs::write(&path, "original\n")?;
    let mut app = AppBuilder::new().with_file(&path, None).build()?;

    let mut replacement = tempfile::NamedTempFile::new_in(directory.path())?;
    replacement.write_all(b"atomically replaced\n")?;
    replacement.flush()?;
    replacement.persist(&path)?;

    wait_until(&mut app, |app| {
        *doc!(app.editor).text() == "atomically replaced\n"
    })
    .await?;

    assert!(!doc!(app.editor).is_modified());
    assert!(app.close().await.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn auto_reload_can_be_disabled() -> anyhow::Result<()> {
    let file = temp_file_with_contents("original\n")?;
    let mut config = test_config();
    config.editor.auto_reload = false;
    let mut app = AppBuilder::new()
        .with_file(file.path(), None)
        .with_config(config)
        .build()?;

    fs::write(file.path(), "external change\n")?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(*doc!(app.editor).text(), "original\n");
    assert!(app.close().await.is_empty());
    Ok(())
}
