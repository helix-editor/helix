use helix_view::graphics::{Color, Modifier, Style, UnderlineStyle};

/// One cell of the terminal. Contains one stylized grapheme.
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
    /// Set the cell's grapheme
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }

    /// Set the cell's grapheme to a [char]
    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        self
    }

    /// Set the foreground [Color]
    pub fn set_fg(&mut self, color: Color) -> &mut Cell {
        self.fg = color;
        self
    }

    /// Set the background [Color]
    pub fn set_bg(&mut self, color: Color) -> &mut Cell {
        self.bg = color;
        self
    }

    /// Set the [Style] of the cell
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

    /// Returns the current style of the cell
    pub fn style(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg)
            .underline_color(self.underline_color)
            .underline_style(self.underline_style)
            .add_modifier(self.modifier)
    }

    /// Resets the cell to a default blank state
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
