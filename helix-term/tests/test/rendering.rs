use super::helpers::*;
use helix_core::{
    diagnostic::{Diagnostic, DiagnosticProvider, Severity},
    Range, Selection,
};
use helix_term::{application::Application, ui::EditorView};
use helix_view::{
    current, current_ref,
    document::DocumentLink,
    graphics::{Color, UnderlineStyle},
};
use tui::buffer::Buffer;

#[cfg(windows)]
use crossterm::event::Event;
#[cfg(not(windows))]
use termina::event::Event;

fn render(app: &Application) -> Buffer {
    let (view, doc) = current_ref!(app.editor);
    let mut surface = Buffer::empty(view.area);
    EditorView::new(Default::default()).render_view(
        &app.editor,
        doc,
        view,
        view.area,
        &mut surface,
        true,
    );
    surface
}

#[tokio::test(flavor = "multi_thread")]
async fn jump_labels_override_document_link_colors() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_input_text("#[| ]#é strings\n")
        .build()?;
    app.editor.set_theme(toml::from_str(
        r##"
        "ui.background" = { fg = "#ffffff", bg = "#000000" }
        "ui.virtual.jump-label" = { fg = "#ffffff", bg = "#00ffff" }
        "markup.link.url" = { fg = "#00ffff", underline = { style = "line" } }
        "ui.selection" = { fg = "#ff00ff", bg = "#ff0000" }
        "ui.cursor" = { fg = "#000000", bg = "#ffffff" }
        "diagnostic" = { underline = { style = "line" } }
        "diagnostic.warning" = { fg = "#00ff00", underline = { style = "curl" } }
        "##,
    )?)?;
    let (view, doc) = current!(app.editor);
    doc.document_links.push(DocumentLink {
        start: 3,
        end: 10,
        link: helix_lsp::lsp::DocumentLink {
            range: helix_lsp::lsp::Range::new(
                helix_lsp::lsp::Position::new(0, 3),
                helix_lsp::lsp::Position::new(0, 10),
            ),
            target: None,
            tooltip: None,
            data: None,
        },
        language_server_id: Default::default(),
    });
    let inner = view.inner_area(doc);
    let x = inner.x + 3;
    let y = inner.y;
    let cyan = Color::Rgb(0, 255, 255);
    let white = Color::Rgb(255, 255, 255);

    let surface = render(&app);
    assert_eq!(surface.get(x, y).unwrap().symbol.as_str(), "s");
    assert_eq!(surface.get(x, y).unwrap().fg, cyan);
    let link_background = surface.get(x, y).unwrap().bg;

    for key in helix_view::input::parse_macro("gw")? {
        app.handle_terminal_events(Ok(Event::Key(key.into()))).await;
    }
    let surface = render(&app);
    for offset in 0..2 {
        let cell = surface.get(x + offset, y).unwrap();
        assert_eq!(cell.symbol.as_str(), "a");
        assert_eq!(cell.fg, white);
        assert_eq!(cell.bg, cyan);
        assert_eq!(cell.underline_style, UnderlineStyle::Line);
    }
    assert_eq!(surface.get(x + 2, y).unwrap().symbol.as_str(), "r");
    assert_eq!(surface.get(x + 2, y).unwrap().fg, cyan);

    // Diagnostics and selections keep their precedence over replacement text.
    let (_, doc) = current!(app.editor);
    doc.replace_diagnostics(
        [Diagnostic {
            range: Range::new(3, 5).into(),
            starts_at_word: true,
            ends_at_word: false,
            zero_width: false,
            line: 0,
            message: "warning".into(),
            severity: Some(Severity::Warning),
            code: None,
            provider: DiagnosticProvider::Lsp {
                server_id: Default::default(),
                identifier: None,
            },
            tags: Vec::new(),
            source: None,
            data: None,
        }],
        &[],
        None,
    );
    let surface = render(&app);
    assert_eq!(surface.get(x, y).unwrap().symbol.as_str(), "a");
    assert_eq!(surface.get(x, y).unwrap().fg, Color::Rgb(0, 255, 0));
    assert_eq!(surface.get(x, y).unwrap().bg, cyan);
    assert_eq!(
        surface.get(x, y).unwrap().underline_style,
        UnderlineStyle::Curl
    );

    let (view, doc) = current!(app.editor);
    doc.set_selection(view.id, Selection::single(3, 6));
    let surface = render(&app);
    assert_eq!(surface.get(x, y).unwrap().symbol.as_str(), "a");
    assert_eq!(surface.get(x, y).unwrap().fg, Color::Rgb(255, 0, 255));
    assert_eq!(surface.get(x, y).unwrap().bg, Color::Rgb(255, 0, 0));
    assert_eq!(
        surface.get(x, y).unwrap().underline_style,
        UnderlineStyle::Curl
    );

    let (view, doc) = current!(app.editor);
    doc.replace_diagnostics([], &[], None);
    doc.set_selection(view.id, Range::point(0).into());

    test_key_sequence(
        &mut app,
        Some("<esc>"),
        Some(&|app| {
            let surface = render(app);
            assert_eq!(surface.get(x, y).unwrap().symbol.as_str(), "s");
            assert_eq!(surface.get(x, y).unwrap().fg, cyan);
            assert_eq!(surface.get(x, y).unwrap().bg, link_background);
        }),
        false,
    )
    .await?;
    Ok(())
}
