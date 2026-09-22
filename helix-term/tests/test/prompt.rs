use super::*;

use helix_core::{regex::Regex, Position};
use helix_term::{
    compositor::{Component, Context, Event},
    job::Jobs,
    ui::{prompt::Movement, Prompt},
};
use helix_view::{graphics::Rect, input::parse_macro};
use tui::buffer::Buffer;

fn render(prompt: &mut Prompt, cx: &mut Context, width: u16) -> (String, Option<Position>) {
    let area = Rect::new(0, 0, width, 1);
    let mut surface = Buffer::empty(area);
    prompt.render(area, &mut surface, cx);
    let text = surface
        .content
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect();
    (text, prompt.cursor(area, cx.editor).0)
}

#[tokio::test(flavor = "multi_thread")]
async fn newline_prompt_preserves_query_and_editing() -> anyhow::Result<()> {
    let mut app = AppBuilder::new().build()?;
    let mut jobs = Jobs::new();
    let mut cx = Context {
        editor: &mut app.editor,
        jobs: &mut jobs,
        scroll: None,
    };
    for ending in ["\n", "\r\n"] {
        let query = format!("dotmap==1\\.3\\.23{ending}");
        let mut prompt = Prompt::new("".into(), None, |_, _| vec![], |_, _, _| {})
            .with_line(query.clone(), cx.editor);
        let (text, cursor) = render(&mut prompt, &mut cx, 40);
        assert_eq!(text.trim_end(), "dotmap==1\\.3\\.23⏎");
        assert_eq!(cursor, Some(Position::new(0, 17)));
        assert_eq!(prompt.line(), &query);
        let matcher = Regex::new(prompt.line())?;
        assert!(matcher.is_match(&format!("dotmap==1.3.23{ending}")));
        assert!(!matcher.is_match("    \"dotmap==1.3.23\","));

        prompt.move_cursor(Movement::BackwardChar(1));
        assert_eq!(
            render(&mut prompt, &mut cx, 40).1,
            Some(Position::new(0, 16))
        );
        prompt.move_cursor(Movement::ForwardChar(1));
        for key in parse_macro("<backspace>")? {
            prompt.handle_event(&Event::Key(key), &mut cx);
        }
        assert_eq!(prompt.line(), "dotmap==1\\.3\\.23");
        let (text, cursor) = render(&mut prompt, &mut cx, 40);
        assert_eq!(text.trim_end(), prompt.line());
        assert_eq!(cursor, Some(Position::new(0, 16)));
        assert!(Regex::new(prompt.line())?.is_match("    \"dotmap==1.3.23\","));
    }
    app.close().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn line_selection_search_register_keeps_newline() -> anyhow::Result<()> {
    test_key_sequence_with_input_text(
        None,
        (
            "#[d|]#otmap==1.3.23\n",
            "x<A-*>",
            "#[|]#",
            LineFeedHandling::AsIs,
        ),
        &|app| {
            assert_eq!(
                app.editor.registers.first('/', &app.editor).unwrap(),
                "dotmap==1\\.3\\.23\n"
            );
        },
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn newline_history_and_paste() -> anyhow::Result<()> {
    let mut app = AppBuilder::new().build()?;
    let mut jobs = Jobs::new();
    let mut cx = Context {
        editor: &mut app.editor,
        jobs: &mut jobs,
        scroll: None,
    };
    let query = "界\r\n\u{301}e\u{301}\nend";
    cx.editor.registers.push('/', query.into())?;
    let mut prompt = Prompt::new("".into(), Some('/'), |_, _| vec![], |_, _, _| {});
    let (text, cursor) = render(&mut prompt, &mut cx, 30);
    // The standalone combining mark after CRLF has no cell of its own.
    assert_eq!(text.trim_end(), "界 ⏎e\u{301}⏎end");
    assert_eq!(cursor, Some(Position::new(0, 0)));
    assert!(prompt.line().is_empty());
    assert_eq!(cx.editor.registers.first('/', cx.editor).unwrap(), query);
    prompt.handle_event(&Event::Paste(query.into()), &mut cx);
    assert_eq!(prompt.line(), query);
    assert_eq!(render(&mut prompt, &mut cx, 30).0, text);
    assert_eq!(
        render(&mut prompt, &mut cx, 30).1,
        Some(Position::new(0, 8))
    );
    prompt.move_start();
    prompt.move_cursor(Movement::ForwardChar(1));
    for key in parse_macro("<del>")? {
        prompt.handle_event(&Event::Key(key), &mut cx);
    }
    assert_eq!(prompt.line(), "界\u{301}e\u{301}\nend");
    assert_eq!(cx.editor.registers.first('/', cx.editor).unwrap(), query);
    app.close().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn newline_scrolling_and_narrow_widths() -> anyhow::Result<()> {
    let mut app = AppBuilder::new().build()?;
    let mut jobs = Jobs::new();
    let mut cx = Context {
        editor: &mut app.editor,
        jobs: &mut jobs,
        scroll: None,
    };
    let mut prompt = Prompt::new("".into(), None, |_, _| vec![], |_, _, _| {})
        .with_line("abc\ndef\r\nghi".into(), cx.editor);
    assert_eq!(
        render(&mut prompt, &mut cx, 8),
        ("…f⏎ghi  ".into(), Some(Position::new(0, 6)))
    );
    prompt.move_start();
    assert_eq!(
        render(&mut prompt, &mut cx, 8),
        ("abc⏎d…  ".into(), Some(Position::new(0, 0)))
    );
    for _ in 0..5 {
        prompt.move_cursor(Movement::ForwardChar(1));
    }
    assert_eq!(render(&mut prompt, &mut cx, 8).1, Some(Position::new(0, 5)));
    prompt.move_cursor(Movement::ForwardChar(1));
    assert_eq!(
        render(&mut prompt, &mut cx, 8),
        ("bc⏎de…  ".into(), Some(Position::new(0, 5)))
    );

    for input in ["\n", "\r\n", "界\n界", "\n\u{301}界\r\nend", "\n\n\n"] {
        for width in 0..=8 {
            prompt.set_line(input.into(), cx.editor);
            for _ in 0..=input.chars().count() {
                let (_, cursor) = render(&mut prompt, &mut cx, width);
                if width <= 2 {
                    assert_eq!(cursor, None);
                } else {
                    assert!(cursor.unwrap().col <= (width - 2) as usize);
                }
                assert_eq!(prompt.line(), input);
                prompt.move_cursor(Movement::BackwardChar(1));
            }
        }
    }
    app.close().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn highlighted_prompt_keeps_existing_newline_layout() -> anyhow::Result<()> {
    let mut app = AppBuilder::new().build()?;
    let mut jobs = Jobs::new();
    let mut prompt = Prompt::new("/".into(), Some('/'), |_, _| vec![], |_, _, _| {})
        .with_language("regex", std::sync::Arc::clone(&app.editor.syn_loader))
        .with_line("a\nb".into(), &app.editor);
    let mut cx = Context {
        editor: &mut app.editor,
        jobs: &mut jobs,
        scroll: None,
    };
    let (text, cursor) = render(&mut prompt, &mut cx, 30);
    assert_eq!(text.trim_end(), "/a");
    assert_eq!(cursor, Some(Position::new(0, 3)));
    assert_eq!(prompt.line(), "a\nb");
    app.close().await;
    Ok(())
}
