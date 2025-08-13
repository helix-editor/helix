use helix_tui::text::{Span, Spans, StyledGrapheme, Text};
use helix_view::graphics::{Color, Modifier, Style};

// Text
#[test]
fn text_width() {
    let text = Text::from("The first line\nThe second line");
    assert_eq!(15, text.width());
}

#[test]
fn text_height() {
    let text = Text::from("The first line\nThe second line");
    assert_eq!(2, text.height());
}

#[test]
fn patch_style() {
    let style1 = Style::default().fg(Color::Yellow);
    let style2 = Style::default().fg(Color::Yellow).bg(Color::Black);
    let mut half_styled_text =
        Text::styled(String::from("The first line\nThe second line"), style1);
    let full_styled_text = Text::styled(String::from("The first line\nThe second line"), style2);
    assert_ne!(half_styled_text, full_styled_text);

    half_styled_text.patch_style(Style::default().bg(Color::Black));
    assert_eq!(half_styled_text, full_styled_text);
}

#[test]
fn set_style() {
    let style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::ITALIC);
    let mut raw_text = Text::raw("The first line\nThe second line");
    let styled_text = Text::styled(String::from("The first line\nThe second line"), style);
    assert_ne!(raw_text, styled_text);

    raw_text.set_style(style);
    assert_eq!(raw_text, styled_text);
}

#[test]
fn text_extend() {
    let style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::ITALIC);
    let mut text = Text::from("The first line\nThe second line");
    assert_eq!(2, text.height());

    // Adding two more unstyled lines
    text.extend(Text::raw("These are two\nmore lines!"));
    assert_eq!(4, text.height());

    // Adding a final two styled lines
    text.extend(Text::styled("Some more lines\nnow with more style!", style));
    assert_eq!(6, text.height());
}

// Span

#[test]
fn styled_graphemes() {
    let style = Style::default().fg(Color::Yellow);
    let span = Span::styled("Text", style);
    let style = Style::default().fg(Color::Green).bg(Color::Black);
    let styled_graphemes = span.styled_graphemes(style);
    assert_eq!(
        vec![
            StyledGrapheme {
                symbol: "T",
                style: Style {
                    fg: Some(Color::Yellow),
                    bg: Some(Color::Black),
                    underline_color: None,
                    underline_style: None,
                    add_modifier: Modifier::empty(),
                    sub_modifier: Modifier::empty(),
                },
            },
            StyledGrapheme {
                symbol: "e",
                style: Style {
                    fg: Some(Color::Yellow),
                    bg: Some(Color::Black),
                    underline_color: None,
                    underline_style: None,
                    add_modifier: Modifier::empty(),
                    sub_modifier: Modifier::empty(),
                },
            },
            StyledGrapheme {
                symbol: "x",
                style: Style {
                    fg: Some(Color::Yellow),
                    bg: Some(Color::Black),
                    underline_color: None,
                    underline_style: None,
                    add_modifier: Modifier::empty(),
                    sub_modifier: Modifier::empty(),
                },
            },
            StyledGrapheme {
                symbol: "t",
                style: Style {
                    fg: Some(Color::Yellow),
                    bg: Some(Color::Black),
                    underline_color: None,
                    underline_style: None,
                    add_modifier: Modifier::empty(),
                    sub_modifier: Modifier::empty(),
                },
            },
        ],
        styled_graphemes.collect::<Vec<StyledGrapheme>>()
    );
}

// Spans

#[test]
fn spans_width() {
    let spans = Spans::from(vec![
        Span::styled("My", Style::default().fg(Color::Yellow)),
        Span::raw(" text"),
    ]);
    assert_eq!(7, spans.width());
}
