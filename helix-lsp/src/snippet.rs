use std::borrow::Cow;

use anyhow::{anyhow, Result};
use helix_core::{smallvec, SmallVec, Tendril};

#[derive(Debug, PartialEq, Eq)]
pub enum CaseChange {
    Upcase,
    Downcase,
    Capitalize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormatItem<'a> {
    Text(Tendril),
    Capture(usize),
    CaseChange(usize, CaseChange),
    Conditional(usize, Option<&'a str>, Option<&'a str>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Regex<'a> {
    value: Tendril,
    replacement: Vec<FormatItem<'a>>,
    options: Tendril,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnippetElement<'a> {
    Tabstop {
        tabstop: usize,
    },
    Placeholder {
        tabstop: usize,
        value: Vec<SnippetElement<'a>>,
    },
    Choice {
        tabstop: usize,
        choices: Vec<Tendril>,
    },
    Variable {
        name: &'a str,
        default: Option<&'a str>,
        regex: Option<Regex<'a>>,
    },
    Text(Tendril),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Snippet<'a> {
    elements: Vec<SnippetElement<'a>>,
}

pub fn parse(s: &str) -> Result<Snippet<'_>> {
    parser::parse(s).map_err(|rest| anyhow!("Failed to parse snippet. Remaining input: {}", rest))
}

fn render_elements(
    snippet_elements: &[SnippetElement<'_>],
    insert: &mut Tendril,
    offset: &mut usize,
    tabstops: &mut Vec<(usize, (usize, usize))>,
    newline_with_offset: &str,
    include_placeholer: bool,
) {
    use SnippetElement::*;

    for element in snippet_elements {
        match element {
            Text(text) => {
                // small optimization to avoid calling replace when it's unnecessary
                let text = if text.contains('\n') {
                    Cow::Owned(text.replace('\n', newline_with_offset))
                } else {
                    Cow::Borrowed(text.as_str())
                };
                *offset += text.chars().count();
                insert.push_str(&text);
            }
            &Variable {
                name: _,
                regex: _,
                r#default,
            } => {
                // TODO: variables. For now, fall back to the default, which defaults to "".
                let text = r#default.unwrap_or_default();
                *offset += text.chars().count();
                insert.push_str(text);
            }
            &Tabstop { tabstop } => {
                tabstops.push((tabstop, (*offset, *offset)));
            }
            Placeholder {
                tabstop,
                value: inner_snippet_elements,
            } => {
                let start_offset = *offset;
                if include_placeholer {
                    render_elements(
                        inner_snippet_elements,
                        insert,
                        offset,
                        tabstops,
                        newline_with_offset,
                        include_placeholer,
                    );
                }
                tabstops.push((*tabstop, (start_offset, *offset)));
            }
            &Choice {
                tabstop,
                choices: _,
            } => {
                // TODO: choices
                tabstops.push((tabstop, (*offset, *offset)));
            }
        }
    }
}

#[allow(clippy::type_complexity)] // only used one time
pub fn render(
    snippet: &Snippet<'_>,
    newline_with_offset: &str,
    include_placeholer: bool,
) -> (Tendril, Vec<SmallVec<[(usize, usize); 1]>>) {
    let mut insert = Tendril::new();
    let mut tabstops = Vec::new();
    let mut offset = 0;

    render_elements(
        &snippet.elements,
        &mut insert,
        &mut offset,
        &mut tabstops,
        newline_with_offset,
        include_placeholer,
    );

    // sort in ascending order (except for 0, which should always be the last one (per lsp doc))
    tabstops.sort_unstable_by_key(|(n, _)| if *n == 0 { usize::MAX } else { *n });

    // merge tabstops with the same index (we take advantage of the fact that we just sorted them
    // above to simply look backwards)
    let mut ntabstops = Vec::<SmallVec<[(usize, usize); 1]>>::new();
    {
        let mut prev = None;
        for (tabstop, r) in tabstops {
            if prev == Some(tabstop) {
                let len_1 = ntabstops.len() - 1;
                ntabstops[len_1].push(r);
            } else {
                prev = Some(tabstop);
                ntabstops.push(smallvec![r]);
            }
        }
    }

    (insert, ntabstops)
}

mod parser {
    use helix_core::Tendril;
    use helix_parsec::*;

    use super::{CaseChange, FormatItem, Regex, Snippet, SnippetElement};

    /*
    https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#snippet_syntax

        any         ::= tabstop | placeholder | choice | variable | text
        tabstop     ::= '$' int | '${' int '}'
        placeholder ::= '${' int ':' any '}'
        choice      ::= '${' int '|' text (',' text)* '|}'
        variable    ::= '$' var | '${' var }'
                        | '${' var ':' any '}'
                        | '${' var '/' regex '/' (format | text)+ '/' options '}'
        format      ::= '$' int | '${' int '}'
                        | '${' int ':' '/upcase' | '/downcase' | '/capitalize' '}'
                        | '${' int ':+' if '}'
                        | '${' int ':?' if ':' else '}'
                        | '${' int ':-' else '}' | '${' int ':' else '}'
        regex       ::= Regular Expression value (ctor-string)
        options     ::= Regular Expression option (ctor-options)
        var         ::= [_a-zA-Z] [_a-zA-Z0-9]*
        int         ::= [0-9]+
        text        ::= .*
        if          ::= text
        else        ::= text
    */

    fn var<'a>() -> impl Parser<'a, Output = &'a str> {
        // var = [_a-zA-Z][_a-zA-Z0-9]*
        move |input: &'a str| match input
            .char_indices()
            .take_while(|(p, c)| {
                *c == '_'
                    || if *p == 0 {
                        c.is_ascii_alphabetic()
                    } else {
                        c.is_ascii_alphanumeric()
                    }
            })
            .last()
        {
            Some((index, c)) if index >= 1 => {
                let index = index + c.len_utf8();
                Ok((&input[index..], &input[0..index]))
            }
            _ => Err(input),
        }
    }

    const TEXT_ESCAPE_CHARS: &[char] = &['\\', '}', '$'];
    const REPLACE_ESCAPE_CHARS: &[char] = &['\\', '}', '$', '/'];
    const CHOICE_TEXT_ESCAPE_CHARS: &[char] = &['\\', '}', '$', '|', ','];

    fn text<'a>(escape_chars: &'static [char]) -> impl Parser<'a, Output = Tendril> {
        move |input: &'a str| {
            let mut chars = input.char_indices();
            let mut res = Tendril::new();
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' => {
                        if let Some((_, c)) = chars.next() {
                            if escape_chars.contains(&c) {
                                res.push(c);
                                continue;
                            }
                        }
                        return Ok((&input[i..], res));
                    }
                    c if escape_chars.contains(&c) => return Ok((&input[i..], res)),
                    c => res.push(c),
                }
            }

            Ok(("", res))
        }
    }

    fn digit<'a>() -> impl Parser<'a, Output = usize> {
        filter_map(take_while(|c| c.is_ascii_digit()), |s| s.parse().ok())
    }

    fn case_change<'a>() -> impl Parser<'a, Output = CaseChange> {
        use CaseChange::*;

        choice!(
            map("upcase", |_| Upcase),
            map("downcase", |_| Downcase),
            map("capitalize", |_| Capitalize),
        )
    }

    fn format<'a>() -> impl Parser<'a, Output = FormatItem<'a>> {
        use FormatItem::*;

        choice!(
            // '$' int
            map(right("$", digit()), Capture),
            // '${' int '}'
            map(seq!("${", digit(), "}"), |seq| Capture(seq.1)),
            // '${' int ':' '/upcase' | '/downcase' | '/capitalize' '}'
            map(seq!("${", digit(), ":/", case_change(), "}"), |seq| {
                CaseChange(seq.1, seq.3)
            }),
            // '${' int ':+' if '}'
            map(
                seq!("${", digit(), ":+", take_until(|c| c == '}'), "}"),
                |seq| { Conditional(seq.1, Some(seq.3), None) }
            ),
            // '${' int ':?' if ':' else '}'
            map(
                seq!(
                    "${",
                    digit(),
                    ":?",
                    take_until(|c| c == ':'),
                    ":",
                    take_until(|c| c == '}'),
                    "}"
                ),
                |seq| { Conditional(seq.1, Some(seq.3), Some(seq.5)) }
            ),
            // '${' int ':-' else '}' | '${' int ':' else '}'
            map(
                seq!(
                    "${",
                    digit(),
                    ":",
                    optional("-"),
                    take_until(|c| c == '}'),
                    "}"
                ),
                |seq| { Conditional(seq.1, None, Some(seq.4)) }
            ),
        )
    }

    fn regex<'a>() -> impl Parser<'a, Output = Regex<'a>> {
        map(
            seq!(
                "/",
                // TODO parse as ECMAScript and convert to rust regex
                non_empty(text(&['/', '\\'])),
                "/",
                one_or_more(choice!(
                    format(),
                    map(text(REPLACE_ESCAPE_CHARS), FormatItem::Text)
                )),
                "/",
                text(&['}', '\\',]),
            ),
            |(_, value, _, replacement, _, options)| Regex {
                value,
                replacement,
                options,
            },
        )
    }

    fn tabstop<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            or(
                right("$", digit()),
                map(seq!("${", digit(), "}"), |values| values.1),
            ),
            |digit| SnippetElement::Tabstop { tabstop: digit },
        )
    }

    fn placeholder<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            seq!(
                "${",
                digit(),
                ":",
                // according to the grammar there is just a single anything here.
                // However in the procese it is explained that placeholders can be nested
                // the example there contains both a placeholder text and a nested placeholder
                // which indicates a list. Looking at the VSCode sourcecode, the placeholder
                // is indeed parsed as zero_or_more so the grammar is simply incorrect here
                zero_or_more(anything(TEXT_ESCAPE_CHARS)),
                "}"
            ),
            |seq| SnippetElement::Placeholder {
                tabstop: seq.1,
                value: seq.3,
            },
        )
    }

    fn choice<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            seq!(
                "${",
                digit(),
                "|",
                sep(text(CHOICE_TEXT_ESCAPE_CHARS), ","),
                "|}",
            ),
            |seq| SnippetElement::Choice {
                tabstop: seq.1,
                choices: seq.3,
            },
        )
    }

    fn variable<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        choice!(
            // $var
            map(right("$", var()), |name| SnippetElement::Variable {
                name,
                default: None,
                regex: None,
            }),
            // ${var:default}
            map(
                seq!("${", var(), ":", take_until(|c| c == '}'), "}",),
                |values| SnippetElement::Variable {
                    name: values.1,
                    default: Some(values.3),
                    regex: None,
                }
            ),
            // ${var/value/format/options}
            map(seq!("${", var(), regex(), "}"), |values| {
                SnippetElement::Variable {
                    name: values.1,
                    default: None,
                    regex: Some(values.2),
                }
            }),
        )
    }

    fn anything<'a>(escape_chars: &'static [char]) -> impl Parser<'a, Output = SnippetElement<'a>> {
        move |input: &'a str| {
            let parser = choice!(
                tabstop(),
                placeholder(),
                choice(),
                variable(),
                map(text(escape_chars), SnippetElement::Text)
            );
            parser.parse(input)
        }
    }

    fn snippet<'a>() -> impl Parser<'a, Output = Snippet<'a>> {
        map(one_or_more(anything(TEXT_ESCAPE_CHARS)), |parts| Snippet {
            elements: parts,
        })
    }

    pub fn parse(s: &str) -> Result<Snippet, &str> {
        snippet().parse(s).and_then(|(remainder, snippet)| {
            if remainder.is_empty() {
                Ok(snippet)
            } else {
                Err(remainder)
            }
        })
    }

    #[cfg(test)]
    mod test {
        use super::SnippetElement::*;
        use super::*;

        #[test]
        fn empty_string_is_error() {
            assert_eq!(Err(""), parse(""));
        }

        #[test]
        fn parse_placeholders_in_function_call() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![
                        Text("match(".into()),
                        Placeholder {
                            tabstop: 1,
                            value: vec!(Text("Arg1".into())),
                        },
                        Text(")".into())
                    ]
                }),
                parse("match(${1:Arg1})")
            )
        }

        #[test]
        fn parse_unterminated_placeholder_error() {
            assert_eq!(Err("${1:)"), parse("match(${1:)"))
        }

        #[test]
        fn parse_empty_placeholder() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![
                        Text("match(".into()),
                        Placeholder {
                            tabstop: 1,
                            value: vec![],
                        },
                        Text(")".into())
                    ]
                }),
                parse("match(${1:})")
            )
        }

        #[test]
        fn parse_placeholders_in_statement() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![
                        Text("local ".into()),
                        Placeholder {
                            tabstop: 1,
                            value: vec!(Text("var".into())),
                        },
                        Text(" = ".into()),
                        Placeholder {
                            tabstop: 1,
                            value: vec!(Text("value".into())),
                        },
                    ]
                }),
                parse("local ${1:var} = ${1:value}")
            )
        }

        #[test]
        fn parse_tabstop_nested_in_placeholder() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![Placeholder {
                        tabstop: 1,
                        value: vec!(Text("var, ".into()), Tabstop { tabstop: 2 },),
                    },]
                }),
                parse("${1:var, $2}")
            )
        }

        #[test]
        fn parse_placeholder_nested_in_placeholder() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![Placeholder {
                        tabstop: 1,
                        value: vec!(
                            Text("foo ".into()),
                            Placeholder {
                                tabstop: 2,
                                value: vec!(Text("bar".into())),
                            },
                        ),
                    },]
                }),
                parse("${1:foo ${2:bar}}")
            )
        }

        #[test]
        fn parse_all() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![
                        Text("hello ".into()),
                        Tabstop { tabstop: 1 },
                        Tabstop { tabstop: 2 },
                        Text(" ".into()),
                        Choice {
                            tabstop: 1,
                            choices: vec!["one".into(), "two".into(), "three".into()]
                        },
                        Text(" ".into()),
                        Variable {
                            name: "name",
                            default: Some("foo"),
                            regex: None
                        },
                        Text(" ".into()),
                        Variable {
                            name: "var",
                            default: None,
                            regex: None
                        },
                        Text(" ".into()),
                        Variable {
                            name: "TM",
                            default: None,
                            regex: None
                        },
                    ]
                }),
                parse("hello $1${2} ${1|one,two,three|} ${name:foo} $var $TM")
            );
        }

        #[test]
        fn regex_capture_replace() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![Variable {
                        name: "TM_FILENAME",
                        default: None,
                        regex: Some(Regex {
                            value: "(.*).+$".into(),
                            replacement: vec![FormatItem::Capture(1)],
                            options: Tendril::new(),
                        }),
                    }]
                }),
                parse("${TM_FILENAME/(.*).+$/$1/}")
            );
        }
    }
}
