use std::borrow::Cow;

use helix_view::graphics::Style;

use crate::text::{Span, Spans};

/// A string split over multiple lines where each line is composed of several clusters, each with
/// their own style.
///
/// A [`Text`], like a [`Span`], can be constructed using one of the many `From` implementations
/// or via the [`Text::raw`] and [`Text::styled`] methods. Helpfully, [`Text`] also implements
/// [`core::iter::Extend`] which enables the concatenation of several [`Text`] blocks.
///
/// ```rust
/// # use helix_tui::text::Text;
/// # use helix_view::graphics::{Color, Modifier, Style};
/// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
///
/// // An initial two lines of `Text` built from a `&str`
/// let mut text = Text::from("The first line\nThe second line");
/// assert_eq!(2, text.height());
///
/// // Adding two more unstyled lines
/// text.extend(Text::raw("These are two\nmore lines!"));
/// assert_eq!(4, text.height());
///
/// // Adding a final two styled lines
/// text.extend(Text::styled("Some more lines\nnow with more style!", style));
/// assert_eq!(6, text.height());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Text<'a> {
    pub lines: Vec<Spans<'a>>,
}

impl<'a> Text<'a> {
    /// Create some text (potentially multiple lines) with no style.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_tui::text::Text;
    /// Text::raw("The first line\nThe second line");
    /// Text::raw(String::from("The first line\nThe second line"));
    /// ```
    pub fn raw<T>(content: T) -> Text<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Text {
            lines: match content.into() {
                Cow::Borrowed(s) => s.lines().map(Spans::from).collect(),
                Cow::Owned(s) => s.lines().map(|l| Spans::from(l.to_owned())).collect(),
            },
        }
    }

    /// Create some text (potentially multiple lines) with a style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use helix_tui::text::Text;
    /// # use helix_view::graphics::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// Text::styled("The first line\nThe second line", style);
    /// Text::styled(String::from("The first line\nThe second line"), style);
    /// ```
    pub fn styled<T>(content: T, style: Style) -> Text<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let mut text = Text::raw(content);
        text.patch_style(style);
        text
    }

    /// Returns the max width of all the lines.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use helix_tui::text::Text;
    /// let text = Text::from("The first line\nThe second line");
    /// assert_eq!(15, text.width());
    /// ```
    pub fn width(&self) -> usize {
        self.lines
            .iter()
            .map(Spans::width)
            .max()
            .unwrap_or_default()
    }

    /// Returns the height.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use helix_tui::text::Text;
    /// let text = Text::from("The first line\nThe second line");
    /// assert_eq!(2, text.height());
    /// ```
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Patch text with a new style. Only updates fields that are in the new style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use helix_tui::text::Text;
    /// # use helix_view::graphics::{Color,  Style};
    /// let style1 = Style::default().fg(Color::Yellow);
    /// let style2 = Style::default().fg(Color::Yellow).bg(Color::Black);
    /// let mut half_styled_text = Text::styled(String::from("The first line\nThe second line"), style1);
    /// let full_styled_text = Text::styled(String::from("The first line\nThe second line"), style2);
    /// assert_ne!(half_styled_text, full_styled_text);
    ///
    /// half_styled_text.patch_style(Style::default().bg(Color::Black));
    /// assert_eq!(half_styled_text, full_styled_text);
    /// ```
    pub fn patch_style(&mut self, style: Style) {
        for line in &mut self.lines {
            for span in &mut line.0 {
                span.style = span.style.patch(style);
            }
        }
    }

    /// Apply a new style to existing text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use helix_tui::text::Text;
    /// # use helix_view::graphics::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// let mut raw_text = Text::raw("The first line\nThe second line");
    /// let styled_text = Text::styled(String::from("The first line\nThe second line"), style);
    /// assert_ne!(raw_text, styled_text);
    ///
    /// raw_text.set_style(style);
    /// assert_eq!(raw_text, styled_text);
    /// ```
    pub fn set_style(&mut self, style: Style) {
        for line in &mut self.lines {
            for span in &mut line.0 {
                span.style = style;
            }
        }
    }
}

impl<'a> From<String> for Text<'a> {
    fn from(s: String) -> Text<'a> {
        Text::raw(s)
    }
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(s: &'a str) -> Text<'a> {
        Text::raw(s)
    }
}

impl<'a> From<Cow<'a, str>> for Text<'a> {
    fn from(s: Cow<'a, str>) -> Text<'a> {
        Text::raw(s)
    }
}

impl<'a> From<Span<'a>> for Text<'a> {
    fn from(span: Span<'a>) -> Text<'a> {
        Text {
            lines: vec![Spans::from(span)],
        }
    }
}

impl<'a> From<Spans<'a>> for Text<'a> {
    fn from(spans: Spans<'a>) -> Text<'a> {
        Text { lines: vec![spans] }
    }
}

impl<'a> From<Vec<Spans<'a>>> for Text<'a> {
    fn from(lines: Vec<Spans<'a>>) -> Text<'a> {
        Text { lines }
    }
}

impl<'a> From<Text<'a>> for String {
    fn from(text: Text<'a>) -> String {
        String::from(&text)
    }
}

impl<'a> From<&Text<'a>> for String {
    fn from(text: &Text<'a>) -> String {
        let size = text
            .lines
            .iter()
            .flat_map(|spans| spans.0.iter().map(|span| span.content.len()))
            .sum::<usize>()
            + text.lines.len().saturating_sub(1); // for newline after each line
        let mut output = String::with_capacity(size);

        for spans in &text.lines {
            if !output.is_empty() {
                output.push('\n');
            }
            for span in &spans.0 {
                output.push_str(&span.content);
            }
        }
        output
    }
}

impl<'a> IntoIterator for Text<'a> {
    type Item = Spans<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

impl<'a> Extend<Spans<'a>> for Text<'a> {
    fn extend<T: IntoIterator<Item = Spans<'a>>>(&mut self, iter: T) {
        self.lines.extend(iter);
    }
}
