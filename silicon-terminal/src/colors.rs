use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::vte::ansi::{Color as AlacColor, NamedColor};
use silicon_view::graphics::{Color, Modifier, Style, UnderlineStyle};

/// Convert an alacritty `Color` to a Silicon `Color`.
pub fn convert_color(color: &AlacColor) -> Color {
    match color {
        AlacColor::Named(named) => convert_named_color(*named),
        AlacColor::Spec(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        AlacColor::Indexed(i) => Color::Indexed(*i),
    }
}

/// Convert alacritty cell flags to Silicon `Modifier` bitflags.
pub fn flags_to_modifier(flags: CellFlags) -> Modifier {
    let mut m = Modifier::empty();
    if flags.contains(CellFlags::BOLD) {
        m.insert(Modifier::BOLD);
    }
    if flags.contains(CellFlags::ITALIC) {
        m.insert(Modifier::ITALIC);
    }
    if flags.contains(CellFlags::DIM) {
        m.insert(Modifier::DIM);
    }
    if flags.contains(CellFlags::INVERSE) {
        m.insert(Modifier::REVERSED);
    }
    if flags.contains(CellFlags::STRIKEOUT) {
        m.insert(Modifier::CROSSED_OUT);
    }
    if flags.contains(CellFlags::HIDDEN) {
        m.insert(Modifier::HIDDEN);
    }
    m
}

/// Convert alacritty cell flags to Silicon `UnderlineStyle`.
pub fn flags_to_underline(flags: CellFlags) -> UnderlineStyle {
    if flags.contains(CellFlags::UNDERCURL) {
        UnderlineStyle::Curl
    } else if flags.contains(CellFlags::DOUBLE_UNDERLINE) {
        UnderlineStyle::DoubleLine
    } else if flags.contains(CellFlags::DOTTED_UNDERLINE) {
        UnderlineStyle::Dotted
    } else if flags.contains(CellFlags::DASHED_UNDERLINE) {
        UnderlineStyle::Dashed
    } else if flags.intersects(CellFlags::ALL_UNDERLINES) {
        UnderlineStyle::Line
    } else {
        UnderlineStyle::Reset
    }
}

/// Build a complete Silicon `Style` from alacritty cell colors and flags.
///
/// If `INVERSE` flag is set, fg and bg are swapped before conversion.
pub fn cell_to_style(
    mut fg: AlacColor,
    mut bg: AlacColor,
    flags: CellFlags,
    underline_color: Option<AlacColor>,
) -> Style {
    // Handle INVERSE flag by swapping fg/bg
    if flags.contains(CellFlags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }

    let modifier = flags_to_modifier(flags) & !Modifier::REVERSED; // We handled INVERSE above
    let underline = flags_to_underline(flags);
    let ul_color = underline_color.map(|c| convert_color(&c));

    Style {
        fg: Some(convert_color(&fg)),
        bg: Some(convert_color(&bg)),
        underline_color: ul_color,
        underline_style: if underline != UnderlineStyle::Reset {
            Some(underline)
        } else {
            None
        },
        add_modifier: modifier,
        sub_modifier: Modifier::empty(),
    }
}

fn convert_named_color(named: NamedColor) -> Color {
    match named {
        NamedColor::Black => Color::Black,
        NamedColor::Red => Color::Red,
        NamedColor::Green => Color::Green,
        NamedColor::Yellow => Color::Yellow,
        NamedColor::Blue => Color::Blue,
        NamedColor::Magenta => Color::Magenta,
        NamedColor::Cyan => Color::Cyan,
        NamedColor::White => Color::LightGray,
        NamedColor::BrightBlack => Color::Gray,
        NamedColor::BrightRed => Color::LightRed,
        NamedColor::BrightGreen => Color::LightGreen,
        NamedColor::BrightYellow => Color::LightYellow,
        NamedColor::BrightBlue => Color::LightBlue,
        NamedColor::BrightMagenta => Color::LightMagenta,
        NamedColor::BrightCyan => Color::LightCyan,
        NamedColor::BrightWhite => Color::White,
        // Dim variants map to their normal counterparts
        NamedColor::DimBlack => Color::Black,
        NamedColor::DimRed => Color::Red,
        NamedColor::DimGreen => Color::Green,
        NamedColor::DimYellow => Color::Yellow,
        NamedColor::DimBlue => Color::Blue,
        NamedColor::DimMagenta => Color::Magenta,
        NamedColor::DimCyan => Color::Cyan,
        NamedColor::DimWhite => Color::LightGray,
        // Foreground / background / cursor use Reset
        NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground => {
            Color::Reset
        }
        NamedColor::Background => Color::Reset,
        NamedColor::Cursor => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::Rgb;

    #[test]
    fn test_named_color_conversion() {
        assert_eq!(convert_color(&AlacColor::Named(NamedColor::Red)), Color::Red);
        assert_eq!(
            convert_color(&AlacColor::Named(NamedColor::BrightBlue)),
            Color::LightBlue
        );
    }

    #[test]
    fn test_rgb_color_conversion() {
        assert_eq!(
            convert_color(&AlacColor::Spec(Rgb { r: 255, g: 128, b: 0 })),
            Color::Rgb(255, 128, 0)
        );
    }

    #[test]
    fn test_indexed_color_conversion() {
        assert_eq!(convert_color(&AlacColor::Indexed(42)), Color::Indexed(42));
    }

    #[test]
    fn test_flags_to_modifier() {
        let flags = CellFlags::BOLD | CellFlags::ITALIC;
        let m = flags_to_modifier(flags);
        assert!(m.contains(Modifier::BOLD));
        assert!(m.contains(Modifier::ITALIC));
        assert!(!m.contains(Modifier::DIM));
    }

    #[test]
    fn test_underline_styles() {
        assert_eq!(flags_to_underline(CellFlags::UNDERCURL), UnderlineStyle::Curl);
        assert_eq!(
            flags_to_underline(CellFlags::DOUBLE_UNDERLINE),
            UnderlineStyle::DoubleLine
        );
        assert_eq!(flags_to_underline(CellFlags::empty()), UnderlineStyle::Reset);
    }

    #[test]
    fn test_inverse_swaps_colors() {
        let fg = AlacColor::Named(NamedColor::Red);
        let bg = AlacColor::Named(NamedColor::Blue);
        let style = cell_to_style(fg, bg, CellFlags::INVERSE, None);
        // After INVERSE, fg should be Blue and bg should be Red
        assert_eq!(style.fg, Some(Color::Blue));
        assert_eq!(style.bg, Some(Color::Red));
    }
}
