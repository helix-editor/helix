use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
/// UNSTABLE
pub enum CursorKind {
    /// █
    Block,
    /// |
    Bar,
    /// _
    Underline,
    /// Hidden cursor, can set cursor position with this to let IME have correct cursor position.
    Hidden,
}

impl Default for CursorKind {
    fn default() -> Self {
        Self::Block
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Margin {
    pub horizontal: u16,
    pub vertical: u16,
}

impl Margin {
    pub fn none() -> Self {
        Self {
            horizontal: 0,
            vertical: 0,
        }
    }

    /// Set uniform margin for all sides.
    pub const fn all(value: u16) -> Self {
        Self {
            horizontal: value,
            vertical: value,
        }
    }

    /// Set the margin of left and right sides to specified value.
    pub const fn horizontal(value: u16) -> Self {
        Self {
            horizontal: value,
            vertical: 0,
        }
    }

    /// Set the margin of top and bottom sides to specified value.
    pub const fn vertical(value: u16) -> Self {
        Self {
            horizontal: 0,
            vertical: value,
        }
    }

    /// Get the total width of the margin (left + right)
    pub const fn width(&self) -> u16 {
        self.horizontal * 2
    }

    /// Get the total height of the margin (top + bottom)
    pub const fn height(&self) -> u16 {
        self.vertical * 2
    }
}

/// A simple rectangle used in the computation of the layout and to give widgets an hint about the
/// area they are supposed to render to. (x, y) = (0, 0) is at the top left corner of the screen.
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    /// Creates a new rect, with width and height
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[inline]
    pub fn area(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    #[inline]
    pub fn left(self) -> u16 {
        self.x
    }

    #[inline]
    pub fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    #[inline]
    pub fn top(self) -> u16 {
        self.y
    }

    #[inline]
    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }

    // Returns a new Rect with width reduced from the left side.
    // This changes the `x` coordinate and clamps it to the right
    // edge of the original Rect.
    pub fn clip_left(self, width: u16) -> Rect {
        let width = std::cmp::min(width, self.width);
        Rect {
            x: self.x.saturating_add(width),
            width: self.width.saturating_sub(width),
            ..self
        }
    }

    // Returns a new Rect with width reduced from the right side.
    // This does _not_ change the `x` coordinate.
    pub fn clip_right(self, width: u16) -> Rect {
        Rect {
            width: self.width.saturating_sub(width),
            ..self
        }
    }

    // Returns a new Rect with height reduced from the top.
    // This changes the `y` coordinate and clamps it to the bottom
    // edge of the original Rect.
    pub fn clip_top(self, height: u16) -> Rect {
        let height = std::cmp::min(height, self.height);
        Rect {
            y: self.y.saturating_add(height),
            height: self.height.saturating_sub(height),
            ..self
        }
    }

    // Returns a new Rect with height reduced from the bottom.
    // This does _not_ change the `y` coordinate.
    pub fn clip_bottom(self, height: u16) -> Rect {
        Rect {
            height: self.height.saturating_sub(height),
            ..self
        }
    }

    pub fn with_height(self, height: u16) -> Rect {
        // new height may make area > u16::max_value, so use new()
        Self::new(self.x, self.y, self.width, height)
    }

    pub fn with_width(self, width: u16) -> Rect {
        Self::new(self.x, self.y, width, self.height)
    }

    pub fn inner(self, margin: Margin) -> Rect {
        if self.width < margin.width() || self.height < margin.height() {
            Rect::default()
        } else {
            Rect {
                x: self.x + margin.horizontal,
                y: self.y + margin.vertical,
                width: self.width - margin.width(),
                height: self.height - margin.height(),
            }
        }
    }

    /// Calculate the union between two [`Rect`]s.
    pub fn union(self, other: Rect) -> Rect {
        // Example:
        //
        // If `Rect` A is positioned at `(0, 0)` with a width and height of `5`,
        // and `Rect` B is positioned at `(5, 0)` with a width and height of `2`,
        // then this is the resulting union:
        //
        // x1 = min(0, 5) => x1 = 0
        // y1 = min(0, 0) => y1 = 0
        // x2 = max(0 + 5, 5 + 2) => x2 = 7
        // y2 = max(0 + 5, 0 + 2) => y2 = 5
        let x1 = min(self.x, other.x);
        let y1 = min(self.y, other.y);
        let x2 = max(self.x + self.width, other.x + other.width);
        let y2 = max(self.y + self.height, other.y + other.height);
        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }

    /// Calculate the intersection between two [`Rect`]s.
    pub fn intersection(self, other: Rect) -> Rect {
        // Example:
        //
        // If `Rect` A is positioned at `(0, 0)` with a width and height of `5`,
        // and `Rect` B is positioned at `(5, 0)` with a width and height of `2`,
        // then this is the resulting intersection:
        //
        // x1 = max(0, 5) => x1 = 5
        // y1 = max(0, 0) => y1 = 0
        // x2 = min(0 + 5, 5 + 2) => x2 = 5
        // y2 = min(0 + 5, 0 + 2) => y2 = 2
        let x1 = max(self.x, other.x);
        let y1 = max(self.y, other.y);
        let x2 = min(self.x + self.width, other.x + other.width);
        let y2 = min(self.y + self.height, other.y + other.height);
        Rect {
            x: x1,
            y: y1,
            width: x2.saturating_sub(x1),
            height: y2.saturating_sub(y1),
        }
    }

    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    LightGray,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl Color {
    /// Creates a `Color` from a hex string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use helix_view::theme::Color;
    ///
    /// let color1 = Color::from_hex("#c0ffee").unwrap();
    /// let color2 = Color::Rgb(192, 255, 238);
    ///
    /// assert_eq!(color1, color2);
    /// ```
    pub fn from_hex(hex: &str) -> Option<Self> {
        if !(hex.starts_with('#') && hex.len() == 7) {
            return None;
        }
        match [1..=2, 3..=4, 5..=6].map(|i| hex.get(i).and_then(|c| u8::from_str_radix(c, 16).ok()))
        {
            [Some(r), Some(g), Some(b)] => Some(Self::Rgb(r, g, b)),
            _ => None,
        }
    }
}

#[cfg(feature = "term")]
impl From<Color> for crossterm::style::Color {
    fn from(color: Color) -> Self {
        use crossterm::style::Color as CColor;

        match color {
            Color::Reset => CColor::Reset,
            Color::Black => CColor::Black,
            Color::Red => CColor::DarkRed,
            Color::Green => CColor::DarkGreen,
            Color::Yellow => CColor::DarkYellow,
            Color::Blue => CColor::DarkBlue,
            Color::Magenta => CColor::DarkMagenta,
            Color::Cyan => CColor::DarkCyan,
            Color::Gray => CColor::DarkGrey,
            Color::LightRed => CColor::Red,
            Color::LightGreen => CColor::Green,
            Color::LightBlue => CColor::Blue,
            Color::LightYellow => CColor::Yellow,
            Color::LightMagenta => CColor::Magenta,
            Color::LightCyan => CColor::Cyan,
            Color::LightGray => CColor::Grey,
            Color::White => CColor::White,
            Color::Indexed(i) => CColor::AnsiValue(i),
            Color::Rgb(r, g, b) => CColor::Rgb { r, g, b },
        }
    }
}

impl Color {
    pub fn red(&self) -> Option<u8> {
        if let Self::Rgb(r, _, _) = self {
            Some(*r)
        } else {
            None
        }
    }

    pub fn green(&self) -> Option<u8> {
        if let Self::Rgb(_, g, _) = self {
            Some(*g)
        } else {
            None
        }
    }

    pub fn blue(&self) -> Option<u8> {
        if let Self::Rgb(_, _, b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle {
    Reset,
    Line,
    Curl,
    Dotted,
    Dashed,
    DoubleLine,
}

impl FromStr for UnderlineStyle {
    type Err = &'static str;

    fn from_str(modifier: &str) -> Result<Self, Self::Err> {
        match modifier {
            "line" => Ok(Self::Line),
            "curl" => Ok(Self::Curl),
            "dotted" => Ok(Self::Dotted),
            "dashed" => Ok(Self::Dashed),
            "double_line" => Ok(Self::DoubleLine),
            _ => Err("Invalid underline style"),
        }
    }
}

#[cfg(feature = "term")]
impl From<UnderlineStyle> for crossterm::style::Attribute {
    fn from(style: UnderlineStyle) -> Self {
        match style {
            UnderlineStyle::Line => crossterm::style::Attribute::Underlined,
            UnderlineStyle::Curl => crossterm::style::Attribute::Undercurled,
            UnderlineStyle::Dotted => crossterm::style::Attribute::Underdotted,
            UnderlineStyle::Dashed => crossterm::style::Attribute::Underdashed,
            UnderlineStyle::DoubleLine => crossterm::style::Attribute::DoubleUnderlined,
            UnderlineStyle::Reset => crossterm::style::Attribute::NoUnderline,
        }
    }
}

bitflags! {
    /// Modifier changes the way a piece of text is displayed.
    ///
    /// They are bitflags so they can easily be composed.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::Modifier;
    ///
    /// let m = Modifier::BOLD | Modifier::ITALIC;
    /// ```
    #[derive(PartialEq, Eq, Debug, Clone, Copy)]
    pub struct Modifier: u16 {
        const BOLD              = 0b0000_0000_0001;
        const DIM               = 0b0000_0000_0010;
        const ITALIC            = 0b0000_0000_0100;
        const SLOW_BLINK        = 0b0000_0001_0000;
        const RAPID_BLINK       = 0b0000_0010_0000;
        const REVERSED          = 0b0000_0100_0000;
        const HIDDEN            = 0b0000_1000_0000;
        const CROSSED_OUT       = 0b0001_0000_0000;
    }
}

impl FromStr for Modifier {
    type Err = &'static str;

    fn from_str(modifier: &str) -> Result<Self, Self::Err> {
        match modifier {
            "bold" => Ok(Self::BOLD),
            "dim" => Ok(Self::DIM),
            "italic" => Ok(Self::ITALIC),
            "slow_blink" => Ok(Self::SLOW_BLINK),
            "rapid_blink" => Ok(Self::RAPID_BLINK),
            "reversed" => Ok(Self::REVERSED),
            "hidden" => Ok(Self::HIDDEN),
            "crossed_out" => Ok(Self::CROSSED_OUT),
            _ => Err("Invalid modifier"),
        }
    }
}

/// Style let you control the main characteristics of the displayed elements.
///
/// ```rust
/// # use helix_view::graphics::{Color, Modifier, Style};
/// Style::default()
///     .fg(Color::Black)
///     .bg(Color::Green)
///     .add_modifier(Modifier::ITALIC | Modifier::BOLD);
/// ```
///
/// It represents an incremental change. If you apply the styles S1, S2, S3 to a cell of the
/// terminal buffer, the style of this cell will be the result of the merge of S1, S2 and S3, not
/// just S3.
///
/// ```rust
/// # use helix_view::graphics::{Rect, Color, UnderlineStyle, Modifier, Style};
/// # use helix_tui::buffer::Buffer;
/// let styles = [
///     Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD | Modifier::ITALIC),
///     Style::default().bg(Color::Red),
///     Style::default().fg(Color::Yellow).remove_modifier(Modifier::ITALIC),
/// ];
/// let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
/// for style in &styles {
///   buffer[(0, 0)].set_style(*style);
/// }
/// assert_eq!(
///     Style {
///         fg: Some(Color::Yellow),
///         bg: Some(Color::Red),
///         add_modifier: Modifier::BOLD,
///         underline_color: Some(Color::Reset),
///         underline_style: Some(UnderlineStyle::Reset),
///         sub_modifier: Modifier::empty(),
///     },
///     buffer[(0, 0)].style(),
/// );
/// ```
///
/// The default implementation returns a `Style` that does not modify anything. If you wish to
/// reset all properties until that point use [`Style::reset`].
///
/// ```
/// # use helix_view::graphics::{Rect, Color, UnderlineStyle, Modifier, Style};
/// # use helix_tui::buffer::Buffer;
/// let styles = [
///     Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD | Modifier::ITALIC),
///     Style::reset().fg(Color::Yellow),
/// ];
/// let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
/// for style in &styles {
///   buffer[(0, 0)].set_style(*style);
/// }
/// assert_eq!(
///     Style {
///         fg: Some(Color::Yellow),
///         bg: Some(Color::Reset),
///         underline_color: Some(Color::Reset),
///         underline_style: Some(UnderlineStyle::Reset),
///         add_modifier: Modifier::empty(),
///         sub_modifier: Modifier::empty(),
///     },
///     buffer[(0, 0)].style(),
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub underline_color: Option<Color>,
    pub underline_style: Option<UnderlineStyle>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    pub const fn new() -> Self {
        Style {
            fg: None,
            bg: None,
            underline_color: None,
            underline_style: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        }
    }

    /// Returns a `Style` resetting all properties.
    pub const fn reset() -> Self {
        Self {
            fg: Some(Color::Reset),
            bg: Some(Color::Reset),
            underline_color: None,
            underline_style: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::all(),
        }
    }

    /// Changes the foreground color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::{Color, Style};
    /// let style = Style::default().fg(Color::Blue);
    /// let diff = Style::default().fg(Color::Red);
    /// assert_eq!(style.patch(diff), Style::default().fg(Color::Red));
    /// ```
    pub const fn fg(mut self, color: Color) -> Style {
        self.fg = Some(color);
        self
    }

    /// Changes the background color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::{Color, Style};
    /// let style = Style::default().bg(Color::Blue);
    /// let diff = Style::default().bg(Color::Red);
    /// assert_eq!(style.patch(diff), Style::default().bg(Color::Red));
    /// ```
    pub const fn bg(mut self, color: Color) -> Style {
        self.bg = Some(color);
        self
    }

    /// Changes the underline color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::{Color, Style};
    /// let style = Style::default().underline_color(Color::Blue);
    /// let diff = Style::default().underline_color(Color::Red);
    /// assert_eq!(style.patch(diff), Style::default().underline_color(Color::Red));
    /// ```
    pub const fn underline_color(mut self, color: Color) -> Style {
        self.underline_color = Some(color);
        self
    }

    /// Changes the underline style.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::{UnderlineStyle, Style};
    /// let style = Style::default().underline_style(UnderlineStyle::Line);
    /// let diff = Style::default().underline_style(UnderlineStyle::Curl);
    /// assert_eq!(style.patch(diff), Style::default().underline_style(UnderlineStyle::Curl));
    /// ```
    pub const fn underline_style(mut self, style: UnderlineStyle) -> Style {
        self.underline_style = Some(style);
        self
    }

    /// Changes the text emphasis.
    ///
    /// When applied, it adds the given modifier to the `Style` modifiers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::{Color, Modifier, Style};
    /// let style = Style::default().add_modifier(Modifier::BOLD);
    /// let diff = Style::default().add_modifier(Modifier::ITALIC);
    /// let patched = style.patch(diff);
    /// assert_eq!(patched.add_modifier, Modifier::BOLD | Modifier::ITALIC);
    /// assert_eq!(patched.sub_modifier, Modifier::empty());
    /// ```
    pub fn add_modifier(mut self, modifier: Modifier) -> Style {
        self.sub_modifier.remove(modifier);
        self.add_modifier.insert(modifier);
        self
    }

    /// Changes the text emphasis.
    ///
    /// When applied, it removes the given modifier from the `Style` modifiers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use helix_view::graphics::{Color, Modifier, Style};
    /// let style = Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC);
    /// let diff = Style::default().remove_modifier(Modifier::ITALIC);
    /// let patched = style.patch(diff);
    /// assert_eq!(patched.add_modifier, Modifier::BOLD);
    /// assert_eq!(patched.sub_modifier, Modifier::ITALIC);
    /// ```
    pub fn remove_modifier(mut self, modifier: Modifier) -> Style {
        self.add_modifier.remove(modifier);
        self.sub_modifier.insert(modifier);
        self
    }

    /// Results in a combined style that is equivalent to applying the two individual styles to
    /// a style one after the other.
    ///
    /// ## Examples
    /// ```
    /// # use helix_view::graphics::{Color, Modifier, Style};
    /// let style_1 = Style::default().fg(Color::Yellow);
    /// let style_2 = Style::default().bg(Color::Red);
    /// let combined = style_1.patch(style_2);
    /// assert_eq!(
    ///     Style::default().patch(style_1).patch(style_2),
    ///     Style::default().patch(combined));
    /// ```
    pub fn patch(mut self, other: Style) -> Style {
        self.fg = other.fg.or(self.fg);
        self.bg = other.bg.or(self.bg);
        self.underline_color = other.underline_color.or(self.underline_color);
        self.underline_style = other.underline_style.or(self.underline_style);

        self.add_modifier.remove(other.sub_modifier);
        self.add_modifier.insert(other.add_modifier);
        self.sub_modifier.remove(other.add_modifier);
        self.sub_modifier.insert(other.sub_modifier);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_size_preservation() {
        for width in 0..256u16 {
            for height in 0..256u16 {
                let rect = Rect::new(0, 0, width, height);
                rect.area(); // Should not panic.
                assert_eq!(rect.width, width);
                assert_eq!(rect.height, height);
            }
        }

        // One dimension below 255, one above. Area below max u16.
        let rect = Rect::new(0, 0, 300, 100);
        assert_eq!(rect.width, 300);
        assert_eq!(rect.height, 100);
    }

    #[test]
    fn test_rect_chop_from_left() {
        let rect = Rect::new(0, 0, 20, 30);
        assert_eq!(Rect::new(10, 0, 10, 30), rect.clip_left(10));
        assert_eq!(
            Rect::new(20, 0, 0, 30),
            rect.clip_left(40),
            "x should be clamped to original width if new width is bigger"
        );
    }

    #[test]
    fn test_rect_chop_from_right() {
        let rect = Rect::new(0, 0, 20, 30);
        assert_eq!(Rect::new(0, 0, 10, 30), rect.clip_right(10));
    }

    #[test]
    fn test_rect_chop_from_top() {
        let rect = Rect::new(0, 0, 20, 30);
        assert_eq!(Rect::new(0, 10, 20, 20), rect.clip_top(10));
        assert_eq!(
            Rect::new(0, 30, 20, 0),
            rect.clip_top(50),
            "y should be clamped to original height if new height is bigger"
        );
    }

    #[test]
    fn test_rect_chop_from_bottom() {
        let rect = Rect::new(0, 0, 20, 30);
        assert_eq!(Rect::new(0, 0, 20, 20), rect.clip_bottom(10));
    }

    fn styles() -> Vec<Style> {
        vec![
            Style::default(),
            Style::default().fg(Color::Yellow),
            Style::default().bg(Color::Yellow),
            Style::default().add_modifier(Modifier::BOLD),
            Style::default().remove_modifier(Modifier::BOLD),
            Style::default().add_modifier(Modifier::ITALIC),
            Style::default().remove_modifier(Modifier::ITALIC),
            Style::default().add_modifier(Modifier::ITALIC | Modifier::BOLD),
            Style::default().remove_modifier(Modifier::ITALIC | Modifier::BOLD),
        ]
    }

    #[test]
    fn combined_patch_gives_same_result_as_individual_patch() {
        let styles = styles();
        for &a in &styles {
            for &b in &styles {
                for &c in &styles {
                    for &d in &styles {
                        let combined = a.patch(b.patch(c.patch(d)));

                        assert_eq!(
                            Style::default().patch(a).patch(b).patch(c).patch(d),
                            Style::default().patch(combined)
                        );
                    }
                }
            }
        }
    }
}
