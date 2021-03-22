use helix_core::hashmap;
use std::collections::HashMap;

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
pub struct Theme {
    scopes: Vec<String>,
    mapping: HashMap<&'static str, Style>,
}

impl Default for Theme {
    fn default() -> Self {
        let mapping = hashmap! {
            "attribute" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "keyword" => Style::default().fg(Color::Rgb(236, 205, 186)), // almond
            "punctuation" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "punctuation.delimiter" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "operator" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "property" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            "variable.parameter" => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
            // TODO distinguish type from type.builtin?
            "type" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "type.builtin" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "constructor" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "function" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "function.macro" => Style::default().fg(Color::Rgb(219, 191, 239)), // lilac
            "comment" => Style::default().fg(Color::Rgb(105, 124, 129)), // sirocco
            "variable.builtin" => Style::default().fg(Color::Rgb(159, 242, 143)), // mint
            "constant" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "constant.builtin" => Style::default().fg(Color::Rgb(255, 255, 255)), // white
            "string" => Style::default().fg(Color::Rgb(204, 204, 204)), // silver
            "escape" => Style::default().fg(Color::Rgb(239, 186, 93)), // honey
            // used for lifetimes
            "label" => Style::default().fg(Color::Rgb(239, 186, 93)), // honey

            // TODO: diferentiate number builtin
            // TODO: diferentiate doc comment
            // TODO: variable as lilac
            // TODO: mod/use statements as white
            // TODO: mod stuff as chamoise
            // TODO: add "(scoped_identifier) @path" for std::mem::
            //
            // concat (ERROR) @syntax-error and "MISSING ;" selectors for errors

            "module" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
            "variable" => Style::default().fg(Color::Rgb(255, 0, 0)), // white
            "function.builtin" => Style::default().fg(Color::Rgb(255, 0, 0)), // white

            "ui.background" => Style::default().bg(Color::Rgb(59, 34, 76)), // midnight
            "ui.linenr" => Style::default().fg(Color::Rgb(90, 89, 119)), // comet
            "ui.statusline" => Style::default().bg(Color::Rgb(40, 23, 51)), // revolver
            "ui.popup" => Style::default().bg(Color::Rgb(40, 23, 51)), // revolver

            "warning" => Style::default().fg(Color::Rgb(255, 205, 28)),
            "error" => Style::default().fg(Color::Rgb(244, 120, 104)),
            "info" => Style::default().fg(Color::Rgb(111, 68, 240)),
            "hint" => Style::default().fg(Color::Rgb(204, 204, 204)),
        };

        let scopes = mapping.keys().map(ToString::to_string).collect();

        Self { scopes, mapping }
    }
}

impl Theme {
    pub fn get(&self, scope: &str) -> Style {
        self.mapping
            .get(scope)
            .copied()
            .unwrap_or_else(|| Style::default().fg(Color::Rgb(0, 0, 255)))
    }

    #[inline]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
}
