use std::borrow::Cow;

use anyhow::{anyhow, Result};
use helix_core::SmallVec;

use crate::{util::lsp_pos_to_pos, OffsetEncoding};

#[derive(Debug, PartialEq, Eq)]
pub enum CaseChange {
    Upcase,
    Downcase,
    Capitalize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormatItem<'a> {
    Text(&'a str),
    Capture(usize),
    CaseChange(usize, CaseChange),
    Conditional(usize, Option<&'a str>, Option<&'a str>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Regex<'a> {
    value: &'a str,
    replacement: Vec<FormatItem<'a>>,
    options: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnippetElement<'a> {
    Tabstop {
        tabstop: usize,
    },
    Placeholder {
        tabstop: usize,
        value: Box<SnippetElement<'a>>,
    },
    Choice {
        tabstop: usize,
        choices: Vec<&'a str>,
    },
    Variable {
        name: &'a str,
        default: Option<&'a str>,
        regex: Option<Regex<'a>>,
    },
    Text(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Snippet<'a> {
    elements: Vec<SnippetElement<'a>>,
}

pub fn parse(s: &str) -> Result<Snippet<'_>> {
    parser::parse(s).map_err(|rest| anyhow!("Failed to parse snippet. Remaining input: {}", rest))
}

pub fn into_transaction<'a>(
    snippet: Snippet<'a>,
    doc: &helix_core::Rope,
    selection: &helix_core::Selection,
    edit: &lsp_types::TextEdit,
    line_ending: &str,
    offset_encoding: OffsetEncoding,
) -> helix_core::Transaction {
    use helix_core::{smallvec, Range, Selection, Transaction};
    use SnippetElement::*;

    let text = doc.slice(..);
    let primary_cursor = selection.primary().cursor(text);

    let start_offset = match lsp_pos_to_pos(doc, edit.range.start, offset_encoding) {
        Some(start) => start as i128 - primary_cursor as i128,
        None => return Transaction::new(doc),
    };
    let end_offset = match lsp_pos_to_pos(doc, edit.range.end, offset_encoding) {
        Some(end) => end as i128 - primary_cursor as i128,
        None => return Transaction::new(doc),
    };

    let newline_with_offset = format!(
        "{line_ending}{blank:width$}",
        width = edit.range.start.character as usize,
        blank = ""
    );

    let mut insert = String::new();
    let mut offset = (primary_cursor as i128 + start_offset) as usize;
    let mut tabstops: Vec<(usize, Range)> = Vec::new();

    for element in snippet.elements {
        match element {
            Text(text) => {
                // small optimization to avoid calling replace when it's unnecessary
                let text = if text.contains('\n') {
                    Cow::Owned(text.replace('\n', &newline_with_offset))
                } else {
                    Cow::Borrowed(text)
                };
                offset += text.chars().count();
                insert.push_str(&text);
            }
            Variable {
                name: _name,
                regex: None,
                r#default,
            } => {
                // TODO: variables. For now, fall back to the default, which defaults to "".
                let text = r#default.unwrap_or_default();
                offset += text.chars().count();
                insert.push_str(text);
            }
            Tabstop { tabstop } => {
                tabstops.push((tabstop, Range::point(offset)));
            }
            Placeholder { tabstop, value } => match value.as_ref() {
                // https://doc.rust-lang.org/beta/unstable-book/language-features/box-patterns.html
                // would make this a bit nicer
                Text(text) => {
                    let len_chars = text.chars().count();
                    tabstops.push((tabstop, Range::new(offset, offset + len_chars + 1)));
                    offset += len_chars;
                    insert.push_str(text);
                }
                other => {
                    log::error!(
                        "Discarding snippet: generating a transaction for placeholder contents {:?} is unimplemented.",
                        other
                    );
                    return Transaction::new(doc);
                }
            },
            other => {
                log::error!(
                    "Discarding snippet: generating a transaction for {:?} is unimplemented.",
                    other
                );
                return Transaction::new(doc);
            }
        }
    }

    let transaction = Transaction::change_by_selection(doc, selection, |range| {
        let cursor = range.cursor(text);
        (
            (cursor as i128 + start_offset) as usize,
            (cursor as i128 + end_offset) as usize,
            Some(insert.clone().into()),
        )
    });

    // sort in ascending order (except for 0, which should always be the last one (per lsp doc))
    tabstops.sort_unstable_by_key(|(n, _range)| if *n == 0 { usize::MAX } else { *n });

    // merge tabstops with the same index (we take advantage of the fact that we just sorted them
    // above to simply look backwards)
    let mut ntabstops = Vec::<SmallVec<[Range; 1]>>::new();
    let mut prev = None;
    for (tabstop, range) in tabstops {
        if prev == Some(tabstop) {
            let len_1 = ntabstops.len() - 1;
            ntabstops[len_1].push(range);
        } else {
            prev = Some(tabstop);
            ntabstops.push(smallvec![range]);
        }
    }

    if let Some(first) = ntabstops.first() {
        transaction.with_selection(Selection::new(first.clone(), 0))
    } else {
        transaction
    }
}

mod parser {
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

    fn text<'a>() -> impl Parser<'a, Output = &'a str> {
        take_while(|c| c != '$')
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
            // Any text
            map(text(), Text),
        )
    }

    fn regex<'a>() -> impl Parser<'a, Output = Regex<'a>> {
        let replacement = reparse_as(take_until(|c| c == '/'), one_or_more(format()));

        map(
            seq!(
                "/",
                take_until(|c| c == '/'),
                "/",
                replacement,
                "/",
                optional(take_until(|c| c == '}')),
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
        // TODO: why doesn't parse_as work?
        // let value = reparse_as(take_until(|c| c == '}'), anything());
        let value = filter_map(take_until(|c| c == '}'), |s| {
            anything().parse(s).map(|parse_result| parse_result.1).ok()
        });

        map(seq!("${", digit(), ":", value, "}"), |seq| {
            SnippetElement::Placeholder {
                tabstop: seq.1,
                value: Box::new(seq.3),
            }
        })
    }

    fn choice<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            seq!(
                "${",
                digit(),
                "|",
                sep(take_until(|c| c == ',' || c == '|'), ","),
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

    fn anything<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        let text = map(text(), SnippetElement::Text);
        choice!(tabstop(), placeholder(), choice(), variable(), text)
    }

    fn snippet<'a>() -> impl Parser<'a, Output = Snippet<'a>> {
        map(one_or_more(anything()), |parts| Snippet { elements: parts })
    }

    pub fn parse(s: &str) -> Result<Snippet, &str> {
        snippet().parse(s).map(|(_input, elements)| elements)
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
                        Text("match("),
                        Placeholder {
                            tabstop: 1,
                            value: Box::new(Text("Arg1")),
                        },
                        Text(")")
                    ]
                }),
                parse("match(${1:Arg1})")
            )
        }

        #[test]
        fn parse_placeholders_in_statement() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![
                        Text("local "),
                        Placeholder {
                            tabstop: 1,
                            value: Box::new(Text("var")),
                        },
                        Text(" = "),
                        Placeholder {
                            tabstop: 1,
                            value: Box::new(Text("value")),
                        },
                    ]
                }),
                parse("local ${1:var} = ${1:value}")
            )
        }

        #[test]
        fn parse_all() {
            assert_eq!(
                Ok(Snippet {
                    elements: vec![
                        Text("hello "),
                        Tabstop { tabstop: 1 },
                        Tabstop { tabstop: 2 },
                        Text(" "),
                        Choice {
                            tabstop: 1,
                            choices: vec!["one", "two", "three"]
                        },
                        Text(" "),
                        Variable {
                            name: "name",
                            default: Some("foo"),
                            regex: None
                        },
                        Text(" "),
                        Variable {
                            name: "var",
                            default: None,
                            regex: None
                        },
                        Text(" "),
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
                            value: "(.*).+$",
                            replacement: vec![FormatItem::Capture(1)],
                            options: None,
                        }),
                    }]
                }),
                parse("${TM_FILENAME/(.*).+$/$1/}")
            );
        }
    }
}
