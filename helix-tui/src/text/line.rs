use std::borrow::Cow;

use crate::text::Span;

/// A string composed of clusters of graphemes, each with their own style.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Spans<'a>(pub Vec<Span<'a>>);

impl Spans<'_> {
    /// Returns the width of the underlying string.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_tui::text::{Span, Spans};
    /// # use helix_view::graphics::{Color, Style};
    /// let spans = Spans::from(vec![
    ///     Span::styled("My", Style::default().fg(Color::Yellow)),
    ///     Span::raw(" text"),
    /// ]);
    /// assert_eq!(7, spans.width());
    /// ```
    pub fn width(&self) -> usize {
        self.0.iter().map(Span::width).sum()
    }
}

impl<'a> From<String> for Spans<'a> {
    fn from(s: String) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}

impl<'a> From<&'a str> for Spans<'a> {
    fn from(s: &'a str) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}

impl<'a> From<Cow<'a, str>> for Spans<'a> {
    fn from(s: Cow<'a, str>) -> Spans<'a> {
        Spans(vec![Span::raw(s)])
    }
}

impl<'a> From<Vec<Span<'a>>> for Spans<'a> {
    fn from(spans: Vec<Span<'a>>) -> Spans<'a> {
        Spans(spans)
    }
}

impl<'a> From<Span<'a>> for Spans<'a> {
    fn from(span: Span<'a>) -> Spans<'a> {
        Spans(vec![span])
    }
}

impl<'a> From<Spans<'a>> for String {
    fn from(line: Spans<'a>) -> String {
        line.0.iter().map(|s| &*s.content).collect()
    }
}

impl<'a> From<&Spans<'a>> for String {
    fn from(line: &Spans<'a>) -> String {
        line.0.iter().map(|s| &*s.content).collect()
    }
}
