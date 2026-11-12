use super::helpers::{self, AppBuilder};

use helix_term::application::Application;
use helix_view::{
    current, current_ref, document::Mode, editor::CompleteAction, input::parse_macro,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[cfg(windows)]
use crossterm::event::{Event, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
#[cfg(not(windows))]
use termina::event::{
    Event, KeyEvent, Modifiers as KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

async fn send_events(app: &mut Application, events: impl IntoIterator<Item = Event>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    for event in events {
        tx.send(Ok(event)).unwrap();
    }
    assert!(
        app.event_loop_until_idle(&mut UnboundedReceiverStream::new(rx))
            .await
    );
}

async fn send_keys(app: &mut Application, keys: &str) {
    send_events(
        app,
        parse_macro(keys)
            .unwrap()
            .into_iter()
            .map(|key| Event::Key(KeyEvent::from(key))),
    )
    .await;
}

async fn check_mouse_switch(
    kind: MouseEventKind,
    gutter: bool,
    separate_documents: bool,
    click_preview: bool,
) -> anyhow::Result<()> {
    // A single file makes path completion deterministic without a language server.
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("hello"), "")?;
    let prefix = "./he";
    let mut config = helpers::test_config();
    config.editor.auto_completion = false;
    config.editor.mouse_yank_register = 'a';
    config.editor.middle_click_paste = true;
    let initial = if separate_documents { "target" } else { prefix };
    let mut app = AppBuilder::new()
        .with_config(config)
        .with_input_text(format!("{initial}#[|\n]#"))
        .build()?;
    app.editor.registers.write('a', vec!["paste".into()])?;
    let target = app.editor.tree.focus;
    if separate_documents {
        send_keys(&mut app, ":vnew<ret>").await;
    }
    let (_, doc) = current!(app.editor);
    doc.set_path(Some(&dir.path().join("buffer")));
    let keys = if separate_documents {
        "i./he<C-x>"
    } else {
        "<C-w>vi<C-x>"
    };
    send_keys(&mut app, keys).await;
    let origin = app.editor.tree.focus;
    assert_ne!(origin, target);
    let (_, doc) = current_ref!(app.editor);
    let origin_selection = doc.selection(origin).clone();

    send_keys(&mut app, "<C-n>").await;
    assert!(matches!(
        app.editor.last_completion,
        Some(CompleteAction::Selected { .. })
    ));
    let (view, doc) = current_ref!(app.editor);
    assert_eq!(view.id, origin);
    assert_eq!(doc.text().to_string(), format!("{prefix}llo\n"));

    let view = app.editor.tree.get(target);
    let doc = app.editor.document(view.doc).unwrap();
    let area = view.inner_area(doc);
    let column = if gutter {
        view.area.x
    } else if click_preview {
        area.x + prefix.chars().count() as u16 + 2
    } else {
        area.x
    };
    send_events(
        &mut app,
        [Event::Mouse(MouseEvent {
            kind,
            column,
            row: area.y,
            modifiers: KeyModifiers::empty(),
        })],
    )
    .await;
    assert_eq!(app.editor.tree.focus, target);

    assert!(app.editor.last_completion.is_none());
    assert!(app
        .editor
        .handlers
        .completions
        .active_completions
        .is_empty());
    assert_eq!(app.editor.mode(), Mode::Normal);
    let pasted = kind == MouseEventKind::Up(MouseButton::Middle);
    let (view, doc) = current_ref!(app.editor);
    let text_after_click = if pasted {
        format!("paste{initial}\n")
    } else {
        format!("{initial}\n")
    };
    assert_eq!(doc.text().to_string(), text_after_click);
    let cursor = if gutter || click_preview {
        initial.chars().count()
    } else {
        0
    };
    assert_eq!(doc.selection(view.id).primary().from(), cursor);
    if !pasted {
        let doc = app
            .editor
            .document(app.editor.tree.get(origin).doc)
            .unwrap();
        assert_eq!(doc.selection(origin), &origin_selection);
    }

    send_keys(&mut app, "iX<esc>").await;
    let (view, doc) = current_ref!(app.editor);
    assert_eq!(view.id, target);
    let expected = if pasted {
        format!("Xpaste{initial}\n")
    } else if gutter || click_preview {
        format!("{initial}X\n")
    } else {
        format!("X{initial}\n")
    };
    assert_eq!(doc.text().to_string(), expected);
    if separate_documents {
        let doc = app
            .editor
            .document(app.editor.tree.get(origin).doc)
            .unwrap();
        assert_eq!(doc.text().to_string(), format!("{prefix}\n"));
    }
    send_keys(&mut app, "u").await;
    let (_, doc) = current_ref!(app.editor);
    assert_eq!(doc.text().to_string(), text_after_click);
    helpers::test_key_sequence(&mut app, Some(":qa!<ret>"), None, true).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn mouse_switch_clears_completion() -> anyhow::Result<()> {
    check_mouse_switch(MouseEventKind::Down(MouseButton::Left), false, false, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn mouse_switch_to_another_document_clears_completion() -> anyhow::Result<()> {
    check_mouse_switch(MouseEventKind::Down(MouseButton::Left), false, true, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn mouse_switch_on_preview_clears_completion() -> anyhow::Result<()> {
    check_mouse_switch(MouseEventKind::Down(MouseButton::Left), false, false, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn gutter_switch_clears_completion() -> anyhow::Result<()> {
    check_mouse_switch(MouseEventKind::Down(MouseButton::Left), true, false, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn right_mouse_switch_clears_completion() -> anyhow::Result<()> {
    check_mouse_switch(MouseEventKind::Up(MouseButton::Right), false, false, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn middle_mouse_switch_clears_completion() -> anyhow::Result<()> {
    check_mouse_switch(MouseEventKind::Up(MouseButton::Middle), false, false, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn scrolling_another_view_keeps_completion() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("hello"), "")?;
    let prefix = "./he";
    let mut config = helpers::test_config();
    config.editor.auto_completion = false;
    let mut app = AppBuilder::new()
        .with_config(config)
        .with_input_text(format!("{prefix}#[|\n]#"))
        .build()?;
    let (_, doc) = current!(app.editor);
    doc.set_path(Some(&dir.path().join("buffer")));
    let target = app.editor.tree.focus;
    send_keys(&mut app, "<C-w>vi<C-x>").await;
    let origin = app.editor.tree.focus;
    send_keys(&mut app, "<C-n>").await;

    let view = app.editor.tree.get(target);
    let area = view.inner_area(app.editor.document(view.doc).unwrap());
    send_events(
        &mut app,
        [Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::empty(),
        })],
    )
    .await;
    assert_eq!(app.editor.tree.focus, origin);
    assert!(matches!(
        app.editor.last_completion,
        Some(CompleteAction::Selected { .. })
    ));
    send_keys(&mut app, "<ret><esc>").await;
    assert!(app.editor.last_completion.is_none());
    let (_, doc) = current_ref!(app.editor);
    assert_eq!(doc.text().to_string(), format!("{prefix}llo\n"));
    helpers::test_key_sequence(&mut app, Some(":qa!<ret>"), None, true).await?;
    Ok(())
}
