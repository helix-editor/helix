use std::borrow::Cow;

use once_cell::sync::Lazy;

use crate::indent::IndentStyle;
use crate::regex::Regex;
use crate::syntax::ModelineConfig;
use crate::{LineEnding, RopeSlice};

// 5 is the vim default
const LINES_TO_CHECK: usize = 5;
const LENGTH_TO_CHECK: usize = 256;

static VIM_MODELINE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\S*\s+)?(vi|[vV]im[<=>]?\d*|ex):\s*(set?\s+)?").unwrap());
static HELIX_MODELINE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\S*\s+)?helix:").unwrap());

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
            // can't guarantee no extra copies, since we need to regex and
            // regexes can't operate over chunks yet, but we can at least
            // limit how much we potentially need to copy because modelines
            // are typically quite short.
            if line.len_chars() > LENGTH_TO_CHECK {
                continue;
            }
            let line = Cow::<str>::from(line);
            modeline.parse_from_line(&line);
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

    fn parse_from_line(&mut self, line: &str) {
        let mut saw_backslash = false;
        let split_modeline = move |c| {
            saw_backslash = match c {
                ':' if !saw_backslash => return true,
                '\\' => true,
                _ => false,
            };
            c == ' ' || c == '\t'
        };

        if let Some(pos) = VIM_MODELINE_REGEX.find(line) {
            for option in line[pos.end()..].split(split_modeline) {
                let parts: Vec<_> = option.split('=').collect();
                match parts[0] {
                    "ft" | "filetype" => {
                        if let Some(val) = parts.get(1) {
                            self.language = Some(val.to_string());
                        }
                    }
                    "sw" | "shiftwidth" => {
                        if let Some(val) = parts.get(1).and_then(|val| val.parse().ok()) {
                            if self.indent_style != Some(IndentStyle::Tabs) {
                                self.indent_style = Some(IndentStyle::Spaces(val));
                            }
                        }
                    }
                    "ff" | "fileformat" => {
                        if let Some(val) = parts.get(1) {
                            self.line_ending = vim_ff_to_helix_line_ending(val);
                        }
                    }
                    "noet" | "noexpandtab" => {
                        self.indent_style = Some(IndentStyle::Tabs);
                    }
                    _ => {}
                }
            }
        }

        if let Some(pos) = HELIX_MODELINE_REGEX.find(line) {
            let config = &line[pos.end()..];
            match toml::from_str::<ModelineConfig>(config) {
                Ok(modeline) => {
                    if let Some(language) = modeline.language {
                        self.language = Some(language);
                    }
                    if let Some(indent) = modeline.indent {
                        self.indent_style = Some(IndentStyle::from_str(&indent.unit));
                    }
                    if let Some(line_ending) = modeline.line_ending {
                        self.line_ending = LineEnding::from_str(&line_ending);
                        if self.line_ending.is_none() {
                            log::warn!("could not interpret line ending {line_ending:?}");
                        }
                    }
                }
                Err(e) => log::warn!("{e}"),
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
            (
                "# helix: language = 'perl'",
                Modeline {
                    language: Some("perl".to_string()),
                    ..Default::default()
                },
            ),
            (
                "# helix: indent = { unit = '   ' }",
                Modeline {
                    indent_style: Some(IndentStyle::Spaces(3)),
                    ..Default::default()
                },
            ),
            (
                "# helix: indent = { unit = \"\t\" }",
                Modeline {
                    indent_style: Some(IndentStyle::Tabs),
                    ..Default::default()
                },
            ),
            (
                "# helix: indent = { unit = \"\\t\" }",
                Modeline {
                    indent_style: Some(IndentStyle::Tabs),
                    ..Default::default()
                },
            ),
            (
                "# helix: line-ending = \"\\r\\n\"",
                Modeline {
                    line_ending: Some(LineEnding::Crlf),
                    ..Default::default()
                },
            ),
        ];
        for (line, expected) in tests {
            let mut got = Modeline::default();
            got.parse_from_line(line);
            assert_eq!(got, expected);
        }
    }
}
