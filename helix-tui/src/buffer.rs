use crate::text::{Span, Spans};
use helix_core::unicode::width::UnicodeWidthStr;
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;

use helix_view::graphics::{Color, Modifier, Rect, Style, UnderlineStyle};

/// A buffer cell
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub underline_color: Color,
    pub underline_style: UnderlineStyle,
    pub modifier: Modifier,
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }

    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        self
    }

    pub fn set_fg(&mut self, color: Color) -> &mut Cell {
        self.fg = color;
        self
    }

    pub fn set_bg(&mut self, color: Color) -> &mut Cell {
        self.bg = color;
        self
    }

    pub fn set_style(&mut self, style: Style) -> &mut Cell {
        if let Some(c) = style.fg {
            self.fg = c;
        }
        if let Some(c) = style.bg {
            self.bg = c;
        }
        if let Some(c) = style.underline_color {
            self.underline_color = c;
        }
        if let Some(style) = style.underline_style {
            self.underline_style = style;
        }

        self.modifier.insert(style.add_modifier);
        self.modifier.remove(style.sub_modifier);
        self
    }

    pub fn style(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg)
            .underline_color(self.underline_color)
            .underline_style(self.underline_style)
            .add_modifier(self.modifier)
    }

    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.underline_color = Color::Reset;
        self.underline_style = UnderlineStyle::Reset;
        self.modifier = Modifier::empty();
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell {
            symbol: " ".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            underline_color: Color::Reset,
            underline_style: UnderlineStyle::Reset,
            modifier: Modifier::empty(),
        }
    }
}

/// A buffer that maps to the desired content of the terminal after the draw call
///
/// No widget in the library interacts directly with the terminal. Instead each of them is required
/// to draw their state to an intermediate buffer. It is basically a grid where each cell contains
/// a grapheme, a foreground color and a background color. This grid will then be used to output
/// the appropriate escape sequences and characters to draw the UI as the user has defined it.
///
/// # Examples:
///
/// ```
/// use helix_tui::buffer::{Buffer, Cell};
/// use helix_view::graphics::{Rect, Color, UnderlineStyle, Style, Modifier};
///
/// let mut buf = Buffer::empty(Rect{x: 0, y: 0, width: 10, height: 5});
/// buf[(0, 2)].set_symbol("x");
/// assert_eq!(buf[(0, 2)].symbol, "x");
/// buf.set_string(3, 0, "string", Style::default().fg(Color::Red).bg(Color::White));
/// assert_eq!(buf[(5, 0)], Cell{
///     symbol: String::from("r"),
///     fg: Color::Red,
///     bg: Color::White,
///     underline_color: Color::Reset,
///     underline_style: UnderlineStyle::Reset,
///     modifier: Modifier::empty(),
/// });
/// buf[(5, 0)].set_char('x');
/// assert_eq!(buf[(5, 0)].symbol, "x");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Buffer {
    /// The area represented by this buffer
    pub area: Rect,
    /// The content of the buffer. The length of this Vec should always be equal to area.width *
    /// area.height
    pub content: Vec<Cell>,
}

impl Buffer {
    /// Returns a Buffer with all cells set to the default one
    pub fn empty(area: Rect) -> Buffer {
        let cell: Cell = Default::default();
        Buffer::filled(area, &cell)
    }

    /// Returns a Buffer with all cells initialized with the attributes of the given Cell
    pub fn filled(area: Rect, cell: &Cell) -> Buffer {
        let size = area.area();
        let mut content = Vec::with_capacity(size);
        for _ in 0..size {
            content.push(cell.clone());
        }
        Buffer { area, content }
    }

    /// Returns a Buffer containing the given lines
    pub fn with_lines<S>(lines: Vec<S>) -> Buffer
    where
        S: AsRef<str>,
    {
        let height = lines.len() as u16;
        let width = lines
            .iter()
            .map(|i| i.as_ref().width() as u16)
            .max()
            .unwrap_or_default();
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width,
            height,
        });
        for (y, line) in lines.iter().enumerate() {
            buffer.set_string(0, y as u16, line, Style::default());
        }
        buffer
    }

    /// Returns the content of the buffer as a slice
    pub fn content(&self) -> &[Cell] {
        &self.content
    }

    /// Returns the area covered by this buffer
    pub fn area(&self) -> &Rect {
        &self.area
    }

    /// Returns a reference to Cell at the given coordinates
    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index_of_opt(x, y).map(|i| &self.content[i])
    }

    /// Returns a mutable reference to Cell at the given coordinates
    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index_of_opt(x, y).map(|i| &mut self.content[i])
    }

    /// Tells whether the global (x, y) coordinates are inside the Buffer's area.
    ///
    /// Global coordinates are offset by the Buffer's area offset (`x`/`y`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use helix_tui::buffer::Buffer;
    /// # use helix_view::graphics::Rect;
    /// let rect = Rect::new(200, 100, 10, 10);
    /// let buffer = Buffer::empty(rect);
    /// // Global coordinates inside the Buffer's area
    /// assert!(buffer.in_bounds(209, 100));
    /// // Global coordinates outside the Buffer's area
    /// assert!(!buffer.in_bounds(210, 100));
    /// ```
    ///
    /// Global coordinates are offset by the Buffer's area offset (`x`/`y`).
    pub fn in_bounds(&self, x: u16, y: u16) -> bool {
        x >= self.area.left()
            && x < self.area.right()
            && y >= self.area.top()
            && y < self.area.bottom()
    }

    /// Returns the index in the Vec<Cell> for the given global (x, y) coordinates.
    ///
    /// Global coordinates are offset by the Buffer's area offset (`x`/`y`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use helix_tui::buffer::Buffer;
    /// # use helix_view::graphics::Rect;
    /// let rect = Rect::new(200, 100, 10, 10);
    /// let buffer = Buffer::empty(rect);
    /// // Global coordinates to the top corner of this Buffer's area
    /// assert_eq!(buffer.index_of(200, 100), 0);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when given an coordinate that is outside of this Buffer's area.
    pub fn index_of(&self, x: u16, y: u16) -> usize {
        debug_assert!(
            self.in_bounds(x, y),
            "Trying to access position outside the buffer: x={}, y={}, area={:?}",
            x,
            y,
            self.area
        );
        ((y - self.area.y) as usize) * (self.area.width as usize) + ((x - self.area.x) as usize)
    }

    /// Returns the index in the Vec<Cell> for the given global (x, y) coordinates,
    /// or `None` if the coordinates are outside the buffer's area.
    fn index_of_opt(&self, x: u16, y: u16) -> Option<usize> {
        if self.in_bounds(x, y) {
            Some(self.index_of(x, y))
        } else {
            None
        }
    }

    /// Returns the (global) coordinates of a cell given its index
    ///
    /// Global coordinates are offset by the Buffer's area offset (`x`/`y`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use helix_tui::buffer::Buffer;
    /// # use helix_view::graphics::Rect;
    /// let rect = Rect::new(200, 100, 10, 10);
    /// let buffer = Buffer::empty(rect);
    /// assert_eq!(buffer.pos_of(0), (200, 100));
    /// assert_eq!(buffer.pos_of(14), (204, 101));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when given an index that is outside the Buffer's content.
    pub fn pos_of(&self, i: usize) -> (u16, u16) {
        debug_assert!(
            i < self.content.len(),
            "Trying to get the coords of a cell outside the buffer: i={} len={}",
            i,
            self.content.len()
        );
        (
            (self.area.x as usize + (i % self.area.width as usize)) as u16,
            (self.area.y as usize + (i / self.area.width as usize)) as u16,
        )
    }

    /// Print a string, starting at the position (x, y)
    pub fn set_string<S>(&mut self, x: u16, y: u16, string: S, style: Style)
    where
        S: AsRef<str>,
    {
        self.set_stringn(x, y, string, usize::MAX, style);
    }

    /// Print at most the first n characters of a string if enough space is available
    /// until the end of the line
    pub fn set_stringn<S>(
        &mut self,
        x: u16,
        y: u16,
        string: S,
        width: usize,
        style: Style,
    ) -> (u16, u16)
    where
        S: AsRef<str>,
    {
        self.set_string_truncated_at_end(x, y, string.as_ref(), width, style)
    }

    /// Print at most the first `width` characters of a string if enough space is available
    /// until the end of the line. If `ellipsis` is true appends a `…` at the end of
    /// truncated lines. If `truncate_start` is `true`, truncate the beginning of the string
    /// instead of the end.
    #[allow(clippy::too_many_arguments)]
    pub fn set_string_truncated(
        &mut self,
        x: u16,
        y: u16,
        string: &str,
        width: usize,
        style: impl Fn(usize) -> Style, // Map a grapheme's string offset to a style
        ellipsis: bool,
        truncate_start: bool,
    ) -> (u16, u16) {
        // prevent panic if out of range
        if !self.in_bounds(x, y) || width == 0 {
            return (x, y);
        }

        let mut index = self.index_of(x, y);
        let mut x_offset = x as usize;
        let width = if ellipsis { width - 1 } else { width };
        let graphemes = string.grapheme_indices(true);
        let max_offset = min(self.area.right() as usize, width.saturating_add(x as usize));
        if !truncate_start {
            for (byte_offset, s) in graphemes {
                let width = s.width();
                if width == 0 {
                    continue;
                }
                // `x_offset + width > max_offset` could be integer overflow on 32-bit machines if we
                // change dimensions to usize or u32 and someone resizes the terminal to 1x2^32.
                if width > max_offset.saturating_sub(x_offset) {
                    break;
                }

                self.content[index].set_symbol(s);
                self.content[index].set_style(style(byte_offset));
                // Reset following cells if multi-width (they would be hidden by the grapheme),
                for i in index + 1..index + width {
                    self.content[i].reset();
                }
                index += width;
                x_offset += width;
            }
            if ellipsis && x_offset - (x as usize) < string.width() {
                self.content[index].set_symbol("…");
            }
        } else {
            let mut start_index = self.index_of(x, y);
            let mut index = self.index_of(max_offset as u16, y);

            let content_width = string.width();
            let truncated = content_width > width;
            if ellipsis && truncated {
                self.content[start_index].set_symbol("…");
                start_index += 1;
            }
            if !truncated {
                index -= width - content_width;
            }
            for (byte_offset, s) in graphemes.rev() {
                let width = s.width();
                if width == 0 {
                    continue;
                }
                let start = index - width;
                if start < start_index {
                    break;
                }
                self.content[start].set_symbol(s);
                self.content[start].set_style(style(byte_offset));
                for i in start + 1..index {
                    self.content[i].reset();
                }
                index -= width;
                x_offset += width;
            }
        }
        (x_offset as u16, y)
    }

    /// Print at most the first `width` characters of a string if enough space is available
    /// until the end of the line.
    pub fn set_string_truncated_at_end(
        &mut self,
        x: u16,
        y: u16,
        string: &str,
        width: usize,
        style: Style,
    ) -> (u16, u16) {
        // prevent panic if out of range
        if !self.in_bounds(x, y) {
            return (x, y);
        }

        let mut index = self.index_of(x, y);
        let mut x_offset = x as usize;
        let max_x_offset = min(self.area.right() as usize, width.saturating_add(x as usize));

        for s in string.graphemes(true) {
            let width = s.width();
            if width == 0 {
                continue;
            }
            // `x_offset + width > max_offset` could be integer overflow on 32-bit machines if we
            // change dimensions to usize or u32 and someone resizes the terminal to 1x2^32.
            if width > max_x_offset.saturating_sub(x_offset) {
                break;
            }

            self.content[index].set_symbol(s);
            self.content[index].set_style(style);
            // Reset following cells if multi-width (they would be hidden by the grapheme),
            for i in index + 1..index + width {
                self.content[i].reset();
            }
            index += width;
            x_offset += width;
        }

        (x_offset as u16, y)
    }

    pub fn set_spans_truncated(&mut self, x: u16, y: u16, spans: &Spans, width: u16) -> (u16, u16) {
        // prevent panic if out of range
        if !self.in_bounds(x, y) || width == 0 {
            return (x, y);
        }

        let mut x_offset = x as usize;
        let max_offset = min(self.area.right(), width.saturating_add(x));
        let mut start_index = self.index_of(x, y);
        let mut index = self.index_of(max_offset, y);

        let content_width = spans.width();
        let truncated = content_width > width as usize;
        if truncated {
            self.content[start_index].set_symbol("…");
            start_index += 1;
        } else {
            index -= width as usize - content_width;
        }
        for span in spans.0.iter().rev() {
            for s in span.content.graphemes(true).rev() {
                let width = s.width();
                if width == 0 {
                    continue;
                }
                let start = index - width;
                if start < start_index {
                    break;
                }
                self.content[start].set_symbol(s);
                self.content[start].set_style(span.style);
                for i in start + 1..index {
                    self.content[i].reset();
                }
                index -= width;
                x_offset += width;
            }
        }
        (x_offset as u16, y)
    }

    pub fn set_spans(&mut self, x: u16, y: u16, spans: &Spans, width: u16) -> (u16, u16) {
        let mut remaining_width = width;
        let mut x = x;
        for span in &spans.0 {
            if remaining_width == 0 {
                break;
            }
            let pos = self.set_stringn(
                x,
                y,
                span.content.as_ref(),
                remaining_width as usize,
                span.style,
            );
            let w = pos.0.saturating_sub(x);
            x = pos.0;
            remaining_width = remaining_width.saturating_sub(w);
        }
        (x, y)
    }

    pub fn set_span(&mut self, x: u16, y: u16, span: &Span, width: u16) -> (u16, u16) {
        self.set_stringn(x, y, span.content.as_ref(), width as usize, span.style)
    }

    #[deprecated(
        since = "0.10.0",
        note = "You should use styling capabilities of `Buffer::set_style`"
    )]
    pub fn set_background(&mut self, area: Rect, color: Color) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                self[(x, y)].set_bg(color);
            }
        }
    }

    pub fn set_style(&mut self, area: Rect, style: Style) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                self[(x, y)].set_style(style);
            }
        }
    }

    /// Resize the buffer so that the mapped area matches the given area and that the buffer
    /// length is equal to area.width * area.height
    pub fn resize(&mut self, area: Rect) {
        let length = area.area();
        if self.content.len() > length {
            self.content.truncate(length);
        } else {
            self.content.resize(length, Default::default());
        }
        self.area = area;
    }

    /// Reset all cells in the buffer
    pub fn reset(&mut self) {
        for c in &mut self.content {
            c.reset();
        }
    }

    /// Clear an area in the buffer
    pub fn clear(&mut self, area: Rect) {
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                self[(x, y)].reset();
            }
        }
    }

    /// Clear an area in the buffer with a default style.
    pub fn clear_with(&mut self, area: Rect, style: Style) {
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                let cell = &mut self[(x, y)];
                cell.reset();
                cell.set_style(style);
            }
        }
    }

    /// Merge an other buffer into this one
    pub fn merge(&mut self, other: &Buffer) {
        let area = self.area.union(other.area);
        let cell: Cell = Default::default();
        self.content.resize(area.area(), cell.clone());

        // Move original content to the appropriate space
        let size = self.area.area();
        for i in (0..size).rev() {
            let (x, y) = self.pos_of(i);
            // New index in content
            let k = ((y - area.y) * area.width + x - area.x) as usize;
            if i != k {
                self.content[k] = self.content[i].clone();
                self.content[i] = cell.clone();
            }
        }

        // Push content of the other buffer into this one (may erase previous
        // data)
        let size = other.area.area();
        for i in 0..size {
            let (x, y) = other.pos_of(i);
            // New index in content
            let k = ((y - area.y) * area.width + x - area.x) as usize;
            self.content[k] = other.content[i].clone();
        }
        self.area = area;
    }

    /// Builds a minimal sequence of coordinates and Cells necessary to update the UI from
    /// self to other.
    ///
    /// We're assuming that buffers are well-formed, that is no double-width cell is followed by
    /// a non-blank cell.
    ///
    /// # Multi-width characters handling:
    ///
    /// ```text
    /// (Index:) `01`
    /// Prev:    `コ`
    /// Next:    `aa`
    /// Updates: `0: a, 1: a'
    /// ```
    ///
    /// ```text
    /// (Index:) `01`
    /// Prev:    `a `
    /// Next:    `コ`
    /// Updates: `0: コ` (double width symbol at index 0 - skip index 1)
    /// ```
    ///
    /// ```text
    /// (Index:) `012`
    /// Prev:    `aaa`
    /// Next:    `aコ`
    /// Updates: `0: a, 1: コ` (double width symbol at index 1 - skip index 2)
    /// ```
    pub fn diff<'a>(&self, other: &'a Buffer) -> Vec<(u16, u16, &'a Cell)> {
        let previous_buffer = &self.content;
        let next_buffer = &other.content;
        let width = self.area.width;

        let mut updates: Vec<(u16, u16, &Cell)> = vec![];
        // Cells invalidated by drawing/replacing preceding multi-width characters:
        let mut invalidated: usize = 0;
        // Cells from the current buffer to skip due to preceding multi-width characters taking their
        // place (the skipped cells should be blank anyway):
        let mut to_skip: usize = 0;
        for (i, (current, previous)) in next_buffer.iter().zip(previous_buffer.iter()).enumerate() {
            if (current != previous || invalidated > 0) && to_skip == 0 {
                let x = (i % width as usize) as u16;
                let y = (i / width as usize) as u16;
                updates.push((x, y, &next_buffer[i]));
            }

            let current_width = current.symbol.width();
            to_skip = current_width.saturating_sub(1);

            let affected_width = std::cmp::max(current_width, previous.symbol.width());
            invalidated = std::cmp::max(affected_width, invalidated).saturating_sub(1);
        }
        updates
    }
}

impl std::ops::Index<(u16, u16)> for Buffer {
    type Output = Cell;

    fn index(&self, (x, y): (u16, u16)) -> &Self::Output {
        let i = self.index_of(x, y);
        &self.content[i]
    }
}

impl std::ops::IndexMut<(u16, u16)> for Buffer {
    fn index_mut(&mut self, (x, y): (u16, u16)) -> &mut Self::Output {
        let i = self.index_of(x, y);
        &mut self.content[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(s: &str) -> Cell {
        let mut cell = Cell::default();
        cell.set_symbol(s);
        cell
    }

    #[test]
    fn it_translates_to_and_from_coordinates() {
        let rect = Rect::new(200, 100, 50, 80);
        let buf = Buffer::empty(rect);

        // First cell is at the upper left corner.
        assert_eq!(buf.pos_of(0), (200, 100));
        assert_eq!(buf.index_of(200, 100), 0);

        // Last cell is in the lower right.
        assert_eq!(buf.pos_of(buf.content.len() - 1), (249, 179));
        assert_eq!(buf.index_of(249, 179), buf.content.len() - 1);
    }

    #[test]
    #[should_panic(expected = "outside the buffer")]
    #[cfg(debug_assertions)]
    fn pos_of_panics_on_out_of_bounds() {
        let rect = Rect::new(0, 0, 10, 10);
        let buf = Buffer::empty(rect);

        // There are a total of 100 cells; zero-indexed means that 100 would be the 101st cell.
        buf.pos_of(100);
    }

    #[test]
    #[should_panic(expected = "outside the buffer")]
    #[cfg(debug_assertions)]
    fn index_of_panics_on_out_of_bounds() {
        let rect = Rect::new(0, 0, 10, 10);
        let buf = Buffer::empty(rect);

        // width is 10; zero-indexed means that 10 would be the 11th cell.
        buf.index_of(10, 0);
    }

    #[test]
    fn buffer_set_string() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buffer = Buffer::empty(area);

        // Zero-width
        buffer.set_stringn(0, 0, "aaa", 0, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["     "]));

        buffer.set_string(0, 0, "aaa", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["aaa  "]));

        // Width limit:
        buffer.set_stringn(0, 0, "bbbbbbbbbbbbbb", 4, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["bbbb "]));

        buffer.set_string(0, 0, "12345", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["12345"]));

        // Width truncation:
        buffer.set_string(0, 0, "123456", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["12345"]));
    }

    #[test]
    fn buffer_set_string_zero_width() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buffer = Buffer::empty(area);

        // Leading grapheme with zero width
        let s = "\u{1}a";
        buffer.set_stringn(0, 0, s, 1, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["a"]));

        // Trailing grapheme with zero with
        let s = "a\u{1}";
        buffer.set_stringn(0, 0, s, 1, Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["a"]));
    }

    #[test]
    fn buffer_set_string_double_width() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buffer = Buffer::empty(area);
        buffer.set_string(0, 0, "コン", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["コン "]));

        // Only 1 space left.
        buffer.set_string(0, 0, "コンピ", Style::default());
        assert_eq!(buffer, Buffer::with_lines(vec!["コン "]));
    }

    #[test]
    fn buffer_with_lines() {
        let buffer =
            Buffer::with_lines(vec!["┌────────┐", "│コンピュ│", "│ーa 上で│", "└────────┘"]);
        assert_eq!(buffer.area.x, 0);
        assert_eq!(buffer.area.y, 0);
        assert_eq!(buffer.area.width, 10);
        assert_eq!(buffer.area.height, 4);
    }

    #[test]
    fn buffer_diffing_empty_empty() {
        let area = Rect::new(0, 0, 40, 40);
        let prev = Buffer::empty(area);
        let next = Buffer::empty(area);
        let diff = prev.diff(&next);
        assert_eq!(diff, vec![]);
    }

    #[test]
    fn buffer_diffing_empty_filled() {
        let area = Rect::new(0, 0, 40, 40);
        let prev = Buffer::empty(area);
        let next = Buffer::filled(area, Cell::default().set_symbol("a"));
        let diff = prev.diff(&next);
        assert_eq!(diff.len(), 40 * 40);
    }

    #[test]
    fn buffer_diffing_filled_filled() {
        let area = Rect::new(0, 0, 40, 40);
        let prev = Buffer::filled(area, Cell::default().set_symbol("a"));
        let next = Buffer::filled(area, Cell::default().set_symbol("a"));
        let diff = prev.diff(&next);
        assert_eq!(diff, vec![]);
    }

    #[test]
    fn buffer_diffing_single_width() {
        let prev = Buffer::with_lines(vec![
            "          ",
            "┌Title─┐  ",
            "│      │  ",
            "│      │  ",
            "└──────┘  ",
        ]);
        let next = Buffer::with_lines(vec![
            "          ",
            "┌TITLE─┐  ",
            "│      │  ",
            "│      │  ",
            "└──────┘  ",
        ]);
        let diff = prev.diff(&next);
        assert_eq!(
            diff,
            vec![
                (2, 1, &cell("I")),
                (3, 1, &cell("T")),
                (4, 1, &cell("L")),
                (5, 1, &cell("E")),
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn buffer_diffing_multi_width() {
        let prev = Buffer::with_lines(vec![
            "┌Title─┐  ",
            "└──────┘  ",
        ]);
        let next = Buffer::with_lines(vec![
            "┌称号──┐  ",
            "└──────┘  ",
        ]);
        let diff = prev.diff(&next);
        assert_eq!(
            diff,
            vec![
                (1, 0, &cell("称")),
                // Skipped "i"
                (3, 0, &cell("号")),
                // Skipped "l"
                (5, 0, &cell("─")),
            ]
        );
    }

    #[test]
    fn buffer_diffing_multi_width_offset() {
        let prev = Buffer::with_lines(vec!["┌称号──┐"]);
        let next = Buffer::with_lines(vec!["┌─称号─┐"]);

        let diff = prev.diff(&next);
        assert_eq!(
            diff,
            vec![(1, 0, &cell("─")), (2, 0, &cell("称")), (4, 0, &cell("号")),]
        );
    }

    #[test]
    fn buffer_merge() {
        let mut one = Buffer::filled(
            Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("1"),
        );
        let two = Buffer::filled(
            Rect {
                x: 0,
                y: 2,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("2"),
        );
        one.merge(&two);
        assert_eq!(one, Buffer::with_lines(vec!["11", "11", "22", "22"]));
    }

    #[test]
    fn buffer_merge2() {
        let mut one = Buffer::filled(
            Rect {
                x: 2,
                y: 2,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("1"),
        );
        let two = Buffer::filled(
            Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("2"),
        );
        one.merge(&two);
        assert_eq!(
            one,
            Buffer::with_lines(vec!["22  ", "22  ", "  11", "  11"])
        );
    }

    #[test]
    fn buffer_merge3() {
        let mut one = Buffer::filled(
            Rect {
                x: 3,
                y: 3,
                width: 2,
                height: 2,
            },
            Cell::default().set_symbol("1"),
        );
        let two = Buffer::filled(
            Rect {
                x: 1,
                y: 1,
                width: 3,
                height: 4,
            },
            Cell::default().set_symbol("2"),
        );
        one.merge(&two);
        let mut merged = Buffer::with_lines(vec!["222 ", "222 ", "2221", "2221"]);
        merged.area = Rect {
            x: 1,
            y: 1,
            width: 4,
            height: 4,
        };
        assert_eq!(one, merged);
    }
}
