use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use toml::Value;

#[cfg(feature = "term")]
pub use tui::style::{Color, Style};

// #[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
// pub struct Color {
//     pub r: u8,
//     pub g: u8,
//     pub b: u8,
// }

// impl Color {
//     pub fn new(r: u8, g: u8, b: u8) -> Self {
//         Self { r, g, b }
//     }
// }

// #[cfg(feature = "term")]
// impl Into<tui::style::Color> for Color {
//     fn into(self) -> tui::style::Color {
//         tui::style::Color::Rgb(self.r, self.g, self.b)
//     }
// }

// impl std::str::FromStr for Color {
//     type Err = ();

//     /// Tries to parse a string (`'#FFFFFF'` or `'FFFFFF'`) into RGB.
//     fn from_str(input: &str) -> Result<Self, Self::Err> {
//         let input = input.trim();
//         let input = match (input.chars().next(), input.len()) {
//             (Some('#'), 7) => &input[1..],
//             (_, 6) => input,
//             _ => return Err(()),
//         };

//         u32::from_str_radix(&input, 16)
//             .map(|s| Color {
//                 r: ((s >> 16) & 0xFF) as u8,
//                 g: ((s >> 8) & 0xFF) as u8,
//                 b: (s & 0xFF) as u8,
//             })
//             .map_err(|_| ())
//     }
// }

// #[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
// pub struct Style {
//     pub fg: Option<Color>,
//     pub bg: Option<Color>,
//     // TODO: modifiers (bold, underline, italic, etc)
// }

// impl Style {
//     pub fn fg(mut self, fg: Color) -> Self {
//         self.fg = Some(fg);
//         self
//     }

//     pub fn bg(mut self, bg: Color) -> Self {
//         self.bg = Some(bg);
//         self
//     }
// }

// #[cfg(feature = "term")]
// impl Into<tui::style::Style> for Style {
//     fn into(self) -> tui::style::Style {
//         let style = tui::style::Style::default();

//         if let Some(fg) = self.fg {
//             style.fg(fg.into());
//         }

//         if let Some(bg) = self.bg {
//             style.bg(bg.into());
//         }

//         style
//     }
// }

/// Color theme for syntax highlighting.
#[derive(Debug)]
pub struct Theme {
    scopes: Vec<String>,
    styles: HashMap<String, Style>,
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Theme, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut styles = HashMap::new();

        if let Ok(colors) = HashMap::<String, Value>::deserialize(deserializer) {
            // scopes.reserve(colors.len());
            styles.reserve(colors.len());
            for (name, style_value) in colors {
                let mut style = Style::default();
                parse_style(&mut style, style_value);
                // scopes.push(name);
                styles.insert(name, style);
            }
        }

        let scopes = styles.keys().map(ToString::to_string).collect();
        Ok(Theme { scopes, styles })
    }
}

fn parse_style(style: &mut Style, value: Value) {
    if let Value::Table(entries) = value {
        for (name, value) in entries {
            match name.as_str() {
                "fg" => {
                    if let Some(color) = parse_color(value) {
                        *style = style.fg(color);
                    }
                }
                "bg" => {
                    if let Some(color) = parse_color(value) {
                        *style = style.bg(color);
                    }
                }
                _ => (),
            }
        }
    } else if let Some(color) = parse_color(value) {
        *style = style.fg(color);
    }
}

fn hex_string_to_rgb(s: &str) -> Option<(u8, u8, u8)> {
    if s.starts_with('#') && s.len() >= 7 {
        if let (Ok(red), Ok(green), Ok(blue)) = (
            u8::from_str_radix(&s[1..3], 16),
            u8::from_str_radix(&s[3..5], 16),
            u8::from_str_radix(&s[5..7], 16),
        ) {
            Some((red, green, blue))
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_color(value: Value) -> Option<Color> {
    if let Value::String(s) = value {
        if let Some((red, green, blue)) = hex_string_to_rgb(&s) {
            Some(Color::Rgb(red, green, blue))
        } else {
            None
        }
    } else {
        None
    }
}

impl Theme {
    pub fn get(&self, scope: &str) -> Style {
        self.styles
            .get(scope)
            .copied()
            .unwrap_or_else(|| Style::default().fg(Color::Rgb(0, 0, 255)))
    }

    #[inline]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
}
