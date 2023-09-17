use std::borrow::Cow;

use helix_stdx::rope::{self, RopeSliceExt};
use once_cell::sync::Lazy;

use crate::indent::IndentStyle;
use crate::{LineEnding, RopeSlice};

// 5 is the vim default
const LINES_TO_CHECK: usize = 5;

static MODELINE_REGEX: Lazy<rope::Regex> =
    Lazy::new(|| rope::Regex::new(r"^(.{0,100}\s{1,100})?helix:").unwrap());
static MODELINE_OPTION_REGEX: Lazy<rope::Regex> =
    Lazy::new(|| rope::Regex::new(r"[a-zA-Z0-9_-]{1,100}(?:=[a-zA-Z0-9_-]{1,100})").unwrap());

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Modeline {
    language: Option<String>,
    indent_style: Option<IndentStyle>,
    line_ending: Option<LineEnding>,
}

impl Modeline {
    pub fn parse(text: RopeSlice) -> Self {
        let mut modeline = Self::default();

        for line in text.lines().take(LINES_TO_CHECK).chain(
            text.lines_at(text.len_lines())
                .reversed()
                .take(LINES_TO_CHECK),
        ) {
            modeline.parse_from_line(line);
        }

        modeline
    }

    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    pub fn indent_style(&self) -> Option<IndentStyle> {
        self.indent_style
    }

    pub fn line_ending(&self) -> Option<LineEnding> {
        self.line_ending
    }

    fn parse_from_line(&mut self, mut line: RopeSlice) {
        if let Some(pos) = MODELINE_REGEX.find(line.regex_input()) {
            line = line.slice(line.byte_to_char(pos.end())..);
            while let Some(opt_pos) = MODELINE_OPTION_REGEX.find(line.regex_input()) {
                let option =
                    Cow::from(line.slice(
                        line.byte_to_char(opt_pos.start())..line.byte_to_char(opt_pos.end()),
                    ));
                let mut parts = option.as_ref().split('=');
                match parts.next().unwrap() {
                    "set-language" | "lang" => {
                        if let Some(val) = parts.next() {
                            self.language = Some(val.to_string());
                        }
                    }
                    "indent-style" => {
                        if let Some(val) = parts.next() {
                            if let Some(indent_style) = IndentStyle::from_option_str(val) {
                                self.indent_style = Some(indent_style);
                            }
                        }
                    }
                    "line-ending" => {
                        if let Some(val) = parts.next() {
                            if let Some(line_ending) = LineEnding::from_option_str(val) {
                                self.line_ending = Some(line_ending);
                            }
                        }
                    }
                    _ => {}
                }
                line = line.slice(line.byte_to_char(opt_pos.end())..);
                let whitespace = line.chars().take_while(|c| char::is_whitespace(*c)).count();
                line = line.slice(whitespace..);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_modeline_parsing() {
        let tests = [
            (
                "# helix: set-language=perl",
                Modeline {
                    language: Some("perl".to_string()),
                    ..Default::default()
                },
            ),
            (
                "# helix: lang=perl",
                Modeline {
                    language: Some("perl".to_string()),
                    ..Default::default()
                },
            ),
            (
                "# helix: indent-style=3",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(3)),
                    ..Default::default()
                },
            ),
            (
                "# helix: indent-style=t",
                Modeline {
                    indent_style: Some(IndentStyle::Tabs),
                    ..Default::default()
                },
            ),
            (
                "# helix: line-ending=crlf",
                Modeline {
                    line_ending: Some(LineEnding::Crlf),
                    ..Default::default()
                },
            ),
            (
                "# helix: lang=perl indent-style=t line-ending=crlf",
                Modeline {
                    language: Some("perl".to_string()),
                    indent_style: Some(IndentStyle::Tabs),
                    line_ending: Some(LineEnding::Crlf),
                },
            ),
            (
                "#//--   helix:   lang=perl   indent-style=t   line-ending=crlf",
                Modeline {
                    language: Some("perl".to_string()),
                    indent_style: Some(IndentStyle::Tabs),
                    line_ending: Some(LineEnding::Crlf),
                },
            ),
        ];
        for (line, expected) in tests {
            let mut got = Modeline::default();
            got.parse_from_line(line.into());
            assert_eq!(got, expected);
        }
    }
}
