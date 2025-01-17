use std::borrow::Cow;

use helix_stdx::rope::{self, RopeSliceExt};
use once_cell::sync::Lazy;

use crate::indent::IndentStyle;
use crate::{LineEnding, RopeSlice};

// 5 is the vim default
const LINES_TO_CHECK: usize = 5;

static HELIX_MODELINE_REGEX: Lazy<rope::Regex> =
    Lazy::new(|| rope::Regex::new(r"^(.{0,100}\s{1,100})?helix:").unwrap());
static HELIX_MODELINE_OPTION_REGEX: Lazy<rope::Regex> =
    Lazy::new(|| rope::Regex::new(r"[a-zA-Z0-9_-]{1,100}(?:=[a-zA-Z0-9_-]{1,100})").unwrap());
static VIM_MODELINE_REGEX: Lazy<rope::Regex> = Lazy::new(|| {
    rope::Regex::new(
        r"^(.{0,100}\s{1,100})?(vi|[vV]im[<=>]?\d{0,100}|ex):\s{0,100}(set?\s{1,100})?",
    )
    .unwrap()
});
static VIM_MODELINE_OPTION_REGEX: Lazy<rope::Regex> =
    Lazy::new(|| rope::Regex::new(r"[a-zA-Z0-9_-]{1,100}(?:=(?:\\:|[^:\s]){0,1000})?").unwrap());

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
        if let Some(pos) = HELIX_MODELINE_REGEX.find(line.regex_input()) {
            line = line.slice(line.byte_to_char(pos.end())..);
            while let Some(opt_pos) = HELIX_MODELINE_OPTION_REGEX.find(line.regex_input()) {
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
        } else if let Some(pos) = VIM_MODELINE_REGEX.find(line.regex_input()) {
            line = line.slice(line.byte_to_char(pos.end())..);
            while let Some(opt_pos) = VIM_MODELINE_OPTION_REGEX.find(line.regex_input()) {
                let option =
                    Cow::from(line.slice(
                        line.byte_to_char(opt_pos.start())..line.byte_to_char(opt_pos.end()),
                    ));
                let mut parts = option.as_ref().splitn(2, '=');
                match parts.next().unwrap() {
                    "ft" | "filetype" => {
                        if let Some(val) = parts.next() {
                            self.language = Some(val.to_string());
                        }
                    }
                    "sw" | "shiftwidth" => {
                        if let Some(val) = parts.next().and_then(|val| val.parse().ok()) {
                            if self.indent_style != Some(IndentStyle::Tabs) {
                                self.indent_style = Some(IndentStyle::Spaces(val));
                            }
                        }
                    }
                    "ff" | "fileformat" => {
                        if let Some(val) = parts.next() {
                            self.line_ending = vim_ff_to_helix_line_ending(val);
                        }
                    }
                    "noet" | "noexpandtab" => {
                        self.indent_style = Some(IndentStyle::Tabs);
                    }
                    "et" | "expandtab" => {
                        if !matches!(self.indent_style, Some(IndentStyle::Spaces(_))) {
                            self.indent_style = Some(IndentStyle::Spaces(0));
                        }
                    }
                    _ => {}
                }
                line = line.slice(line.byte_to_char(opt_pos.end())..);
                let whitespace = line
                    .chars()
                    .take_while(|c| char::is_whitespace(*c) || *c == ':')
                    .count();
                line = line.slice(whitespace..);
            }
        }
    }
}

fn vim_ff_to_helix_line_ending(val: &str) -> Option<LineEnding> {
    match val {
        "dos" => Some(LineEnding::Crlf),
        "unix" => Some(LineEnding::LF),
        #[cfg(feature = "unicode-lines")]
        "mac" => Some(LineEnding::CR),
        _ => None,
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
            (
                "vi:noai:sw=3 ts=6",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(3)),
                    ..Default::default()
                },
            ),
            (
                "vim: tw=77",
                Modeline {
                    ..Default::default()
                },
            ),
            (
                "/* vim: set ai sw=5: */",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(5)),
                    ..Default::default()
                },
            ),
            (
                "# vim: set noexpandtab:",
                Modeline {
                    indent_style: Some(IndentStyle::Tabs),
                    ..Default::default()
                },
            ),
            (
                "# vim: set expandtab:",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(0)),
                    ..Default::default()
                },
            ),
            (
                "// vim: noai:ts=4:sw=4",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(4)),
                    ..Default::default()
                },
            ),
            (
                "/* vim: set noai ts=4 sw=4: */",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(4)),
                    ..Default::default()
                },
            ),
            (
                "/* vim: set fdm=expr ft=c fde=getline(v\\:lnum)=~'{'?'>1'\\:'1' sw=4: */",
                Modeline {
                    language: Some("c".to_string()),
                    indent_style: Some(IndentStyle::Spaces(4)),
                    ..Default::default()
                },
            ),
            (
                "/* vim: set ts=8 sw=4 tw=0 noet : */",
                Modeline {
                    indent_style: Some(IndentStyle::Tabs),
                    ..Default::default()
                },
            ),
            (
                "vim:ff=unix ts=4 sw=4",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(4)),
                    line_ending: Some(LineEnding::LF),
                    ..Default::default()
                },
            ),
            (
                "vim:tw=78:sw=2:ts=2:ft=help:norl:nowrap:",
                Modeline {
                    language: Some("help".to_string()),
                    indent_style: Some(IndentStyle::Spaces(2)),
                    ..Default::default()
                },
            ),
            (
                "# vim: ft=zsh sw=2 ts=2 et",
                Modeline {
                    language: Some("zsh".to_string()),
                    indent_style: Some(IndentStyle::Spaces(2)),
                    ..Default::default()
                },
            ),
            (
                "# vim:ft=sh:",
                Modeline {
                    language: Some("sh".to_string()),
                    ..Default::default()
                },
            ),
            (
                "\" vim:ts=8:sts=4:sw=4:expandtab:ft=vim",
                Modeline {
                    language: Some("vim".to_string()),
                    indent_style: Some(IndentStyle::Spaces(4)),
                    ..Default::default()
                },
            ),
            (
                "\" vim: ts=8 noet tw=100 sw=8 sts=0 ft=vim isk+=-",
                Modeline {
                    language: Some("vim".to_string()),
                    indent_style: Some(IndentStyle::Tabs),
                    ..Default::default()
                },
            ),
            (
                "; vim:ft=gitconfig:",
                Modeline {
                    language: Some("gitconfig".to_string()),
                    ..Default::default()
                },
            ),
        ];
        for (line, expected) in tests {
            let mut got = Modeline::default();
            got.parse_from_line(line.into());
            assert_eq!(got, expected, "{line}");
        }
    }
}
