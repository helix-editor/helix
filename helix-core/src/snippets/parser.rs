/*!
A parser for LSP/VSCode style snippet syntax
See <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#snippet_syntax>.

``` text
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
```
*/

use crate::Tendril;
use helix_parsec::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CaseChange {
    Upcase,
    Downcase,
    Capitalize,
    PascalCase,
    CamelCase,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormatItem {
    Text(Tendril),
    Capture(usize),
    CaseChange(usize, CaseChange),
    Conditional(usize, Tendril, Tendril),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Transform {
    pub regex: Tendril,
    pub replacement: Vec<FormatItem>,
    pub options: Tendril,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnippetElement {
    Tabstop {
        tabstop: usize,
        transform: Option<Transform>,
    },
    Placeholder {
        tabstop: usize,
        value: Vec<SnippetElement>,
    },
    Choice {
        tabstop: usize,
        choices: Vec<Tendril>,
    },
    Variable {
        name: Tendril,
        default: Option<Vec<SnippetElement>>,
        transform: Option<Transform>,
    },
    Text(Tendril),
}

pub fn parse(s: &str) -> Result<Vec<SnippetElement>, &str> {
    snippet().parse(s).and_then(|(remainder, snippet)| {
        if remainder.is_empty() {
            Ok(snippet)
        } else {
            Err(remainder)
        }
    })
}

fn var<'a>() -> impl Parser<'a, Output = &'a str> {
    // var = [_a-zA-Z][_a-zA-Z0-9]*
    move |input: &'a str| {
        input
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
            .map(|(index, c)| {
                let index = index + c.len_utf8();
                (&input[index..], &input[0..index])
            })
            .ok_or(input)
    }
}

const TEXT_ESCAPE_CHARS: &[char] = &['\\', '}', '$'];
const CHOICE_TEXT_ESCAPE_CHARS: &[char] = &['\\', '|', ','];

fn text<'a>(
    escape_chars: &'static [char],
    term_chars: &'static [char],
) -> impl Parser<'a, Output = Tendril> {
    move |input: &'a str| {
        let mut chars = input.char_indices().peekable();
        let mut res = Tendril::new();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => {
                    if let Some(&(_, c)) = chars.peek() {
                        if escape_chars.contains(&c) {
                            chars.next();
                            res.push(c);
                            continue;
                        }
                    }
                    res.push('\\');
                }
                c if term_chars.contains(&c) => return Ok((&input[i..], res)),
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
        map("pascalcase", |_| PascalCase),
        map("camelcase", |_| CamelCase),
    )
}

fn format<'a>() -> impl Parser<'a, Output = FormatItem> {
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
            seq!("${", digit(), ":+", text(TEXT_ESCAPE_CHARS, &['}']), "}"),
            |seq| { Conditional(seq.1, seq.3, Tendril::new()) }
        ),
        // '${' int ':?' if ':' else '}'
        map(
            seq!(
                "${",
                digit(),
                ":?",
                text(TEXT_ESCAPE_CHARS, &[':']),
                ":",
                text(TEXT_ESCAPE_CHARS, &['}']),
                "}"
            ),
            |seq| { Conditional(seq.1, seq.3, seq.5) }
        ),
        // '${' int ':-' else '}' | '${' int ':' else '}'
        map(
            seq!(
                "${",
                digit(),
                ":",
                optional("-"),
                text(TEXT_ESCAPE_CHARS, &['}']),
                "}"
            ),
            |seq| { Conditional(seq.1, Tendril::new(), seq.4) }
        ),
    )
}

fn regex<'a>() -> impl Parser<'a, Output = Transform> {
    map(
        seq!(
            "/",
            // TODO parse as ECMAScript and convert to rust regex
            text(&['/'], &['/']),
            "/",
            zero_or_more(choice!(
                format(),
                // text doesn't parse $, if format fails we just accept the $ as text
                map("$", |_| FormatItem::Text("$".into())),
                map(text(&['\\', '/'], &['/', '$']), FormatItem::Text),
            )),
            "/",
            // vscode really doesn't allow escaping } here
            // so it's impossible to write a regex escape containing a }
            // we can consider deviating here and allowing the escape
            text(&[], &['}']),
        ),
        |(_, value, _, replacement, _, options)| Transform {
            regex: value,
            replacement,
            options,
        },
    )
}

fn tabstop<'a>() -> impl Parser<'a, Output = SnippetElement> {
    map(
        or(
            map(right("$", digit()), |i| (i, None)),
            map(
                seq!("${", digit(), optional(regex()), "}"),
                |(_, i, transform, _)| (i, transform),
            ),
        ),
        |(tabstop, transform)| SnippetElement::Tabstop { tabstop, transform },
    )
}

fn placeholder<'a>() -> impl Parser<'a, Output = SnippetElement> {
    map(
        seq!(
            "${",
            digit(),
            ":",
            // according to the grammar there is just a single anything here.
            // However in the prose it is explained that placeholders can be nested.
            // The example there contains both a placeholder text and a nested placeholder
            // which indicates a list. Looking at the VSCode sourcecode, the placeholder
            // is indeed parsed as zero_or_more so the grammar is simply incorrect here
            zero_or_more(anything(TEXT_ESCAPE_CHARS, true)),
            "}"
        ),
        |seq| SnippetElement::Placeholder {
            tabstop: seq.1,
            value: seq.3,
        },
    )
}

fn choice<'a>() -> impl Parser<'a, Output = SnippetElement> {
    map(
        seq!(
            "${",
            digit(),
            "|",
            sep(text(CHOICE_TEXT_ESCAPE_CHARS, &['|', ',']), ","),
            "|}",
        ),
        |seq| SnippetElement::Choice {
            tabstop: seq.1,
            choices: seq.3,
        },
    )
}

fn variable<'a>() -> impl Parser<'a, Output = SnippetElement> {
    choice!(
        // $var
        map(right("$", var()), |name| SnippetElement::Variable {
            name: name.into(),
            default: None,
            transform: None,
        }),
        // ${var}
        map(seq!("${", var(), "}",), |values| SnippetElement::Variable {
            name: values.1.into(),
            default: None,
            transform: None,
        }),
        // ${var:default}
        map(
            seq!(
                "${",
                var(),
                ":",
                zero_or_more(anything(TEXT_ESCAPE_CHARS, true)),
                "}",
            ),
            |values| SnippetElement::Variable {
                name: values.1.into(),
                default: Some(values.3),
                transform: None,
            }
        ),
        // ${var/value/format/options}
        map(seq!("${", var(), regex(), "}"), |values| {
            SnippetElement::Variable {
                name: values.1.into(),
                default: None,
                transform: Some(values.2),
            }
        }),
    )
}

fn anything<'a>(
    escape_chars: &'static [char],
    end_at_brace: bool,
) -> impl Parser<'a, Output = SnippetElement> {
    let term_chars: &[_] = if end_at_brace { &['$', '}'] } else { &['$'] };
    move |input: &'a str| {
        let parser = choice!(
            tabstop(),
            placeholder(),
            choice(),
            variable(),
            map("$", |_| SnippetElement::Text("$".into())),
            map(text(escape_chars, term_chars), SnippetElement::Text),
        );
        parser.parse(input)
    }
}

fn snippet<'a>() -> impl Parser<'a, Output = Vec<SnippetElement>> {
    one_or_more(anything(TEXT_ESCAPE_CHARS, false))
}

#[cfg(test)]
mod test {
    use crate::snippets::{Snippet, SnippetRenderCtx};

    use super::SnippetElement::*;
    use super::*;

    #[test]
    fn empty_string_is_error() {
        assert_eq!(Err(""), parse(""));
    }

    #[test]
    fn parse_placeholders_in_function_call() {
        assert_eq!(
            Ok(vec![
                Text("match(".into()),
                Placeholder {
                    tabstop: 1,
                    value: vec![Text("Arg1".into())],
                },
                Text(")".into()),
            ]),
            parse("match(${1:Arg1})")
        )
    }

    #[test]
    fn unterminated_placeholder() {
        assert_eq!(
            Ok(vec![
                Text("match(".into()),
                Text("$".into()),
                Text("{1:)".into())
            ]),
            parse("match(${1:)")
        )
    }

    #[test]
    fn parse_empty_placeholder() {
        assert_eq!(
            Ok(vec![
                Text("match(".into()),
                Placeholder {
                    tabstop: 1,
                    value: vec![],
                },
                Text(")".into()),
            ]),
            parse("match(${1:})")
        )
    }

    #[test]
    fn parse_placeholders_in_statement() {
        assert_eq!(
            Ok(vec![
                Text("local ".into()),
                Placeholder {
                    tabstop: 1,
                    value: vec![Text("var".into())],
                },
                Text(" = ".into()),
                Placeholder {
                    tabstop: 1,
                    value: vec![Text("value".into())],
                },
            ]),
            parse("local ${1:var} = ${1:value}")
        )
    }

    #[test]
    fn parse_tabstop_nested_in_placeholder() {
        assert_eq!(
            Ok(vec![Placeholder {
                tabstop: 1,
                value: vec![
                    Text("var, ".into()),
                    Tabstop {
                        tabstop: 2,
                        transform: None
                    }
                ],
            }]),
            parse("${1:var, $2}")
        )
    }

    #[test]
    fn parse_placeholder_nested_in_placeholder() {
        assert_eq!(
            Ok({
                vec![Placeholder {
                    tabstop: 1,
                    value: vec![
                        Text("foo ".into()),
                        Placeholder {
                            tabstop: 2,
                            value: vec![Text("bar".into())],
                        },
                    ],
                }]
            }),
            parse("${1:foo ${2:bar}}")
        )
    }

    #[test]
    fn parse_all() {
        assert_eq!(
            Ok(vec![
                Text("hello ".into()),
                Tabstop {
                    tabstop: 1,
                    transform: None
                },
                Tabstop {
                    tabstop: 2,
                    transform: None
                },
                Text(" ".into()),
                Choice {
                    tabstop: 1,
                    choices: vec!["one".into(), "two".into(), "three".into()],
                },
                Text(" ".into()),
                Variable {
                    name: "name".into(),
                    default: Some(vec![Text("foo".into())]),
                    transform: None,
                },
                Text(" ".into()),
                Variable {
                    name: "var".into(),
                    default: None,
                    transform: None,
                },
                Text(" ".into()),
                Variable {
                    name: "TM".into(),
                    default: None,
                    transform: None,
                },
            ]),
            parse("hello $1${2} ${1|one,two,three|} ${name:foo} $var $TM")
        );
    }

    #[test]
    fn regex_capture_replace() {
        assert_eq!(
            Ok({
                vec![Variable {
                    name: "TM_FILENAME".into(),
                    default: None,
                    transform: Some(Transform {
                        regex: "(.*).+$".into(),
                        replacement: vec![FormatItem::Capture(1), FormatItem::Text("$".into())],
                        options: Tendril::new(),
                    }),
                }]
            }),
            parse("${TM_FILENAME/(.*).+$/$1$/}")
        );
    }

    #[test]
    fn rust_macro() {
        assert_eq!(
            Ok({
                vec![
                    Text("macro_rules! ".into()),
                    Tabstop {
                        tabstop: 1,
                        transform: None,
                    },
                    Text(" {\n    (".into()),
                    Tabstop {
                        tabstop: 2,
                        transform: None,
                    },
                    Text(") => {\n        ".into()),
                    Tabstop {
                        tabstop: 0,
                        transform: None,
                    },
                    Text("\n    };\n}".into()),
                ]
            }),
            parse("macro_rules! $1 {\n    ($2) => {\n        $0\n    };\n}")
        );
    }

    fn assert_text(snippet: &str, parsed_text: &str) {
        let snippet = Snippet::parse(snippet).unwrap();
        let mut rendered_snippet = snippet.prepare_render();
        let rendered_text = snippet
            .render_at(
                &mut rendered_snippet,
                "".into(),
                false,
                &mut SnippetRenderCtx::test_ctx(),
                0,
            )
            .0;
        assert_eq!(rendered_text, parsed_text)
    }

    #[test]
    fn robust_parsing() {
        assert_text("$", "$");
        assert_text("\\\\$", "\\$");
        assert_text("{", "{");
        assert_text("\\}", "}");
        assert_text("\\abc", "\\abc");
        assert_text("foo${f:\\}}bar", "foo}bar");
        assert_text("\\{", "\\{");
        assert_text("I need \\\\\\$", "I need \\$");
        assert_text("\\", "\\");
        assert_text("\\{{", "\\{{");
        assert_text("{{", "{{");
        assert_text("{{dd", "{{dd");
        assert_text("}}", "}}");
        assert_text("ff}}", "ff}}");
        assert_text("farboo", "farboo");
        assert_text("far{{}}boo", "far{{}}boo");
        assert_text("far{{123}}boo", "far{{123}}boo");
        assert_text("far\\{{123}}boo", "far\\{{123}}boo");
        assert_text("far{{id:bern}}boo", "far{{id:bern}}boo");
        assert_text("far{{id:bern {{basel}}}}boo", "far{{id:bern {{basel}}}}boo");
        assert_text(
            "far{{id:bern {{id:basel}}}}boo",
            "far{{id:bern {{id:basel}}}}boo",
        );
        assert_text(
            "far{{id:bern {{id2:basel}}}}boo",
            "far{{id:bern {{id2:basel}}}}boo",
        );
        assert_text("${}$\\a\\$\\}\\\\", "${}$\\a$}\\");
        assert_text("farboo", "farboo");
        assert_text("far{{}}boo", "far{{}}boo");
        assert_text("far{{123}}boo", "far{{123}}boo");
        assert_text("far\\{{123}}boo", "far\\{{123}}boo");
        assert_text("far`123`boo", "far`123`boo");
        assert_text("far\\`123\\`boo", "far\\`123\\`boo");
        assert_text("\\$far-boo", "$far-boo");
    }

    fn assert_snippet(snippet: &str, expect: &[SnippetElement]) {
        let elements = parse(snippet).unwrap();
        assert_eq!(elements, expect.to_owned())
    }

    #[test]
    fn parse_variable() {
        use SnippetElement::*;
        assert_snippet(
            "$far-boo",
            &[
                Variable {
                    name: "far".into(),
                    default: None,
                    transform: None,
                },
                Text("-boo".into()),
            ],
        );
        assert_snippet(
            "far$farboo",
            &[
                Text("far".into()),
                Variable {
                    name: "farboo".into(),
                    transform: None,
                    default: None,
                },
            ],
        );
        assert_snippet(
            "far${farboo}",
            &[
                Text("far".into()),
                Variable {
                    name: "farboo".into(),
                    transform: None,
                    default: None,
                },
            ],
        );
        assert_snippet(
            "$123",
            &[Tabstop {
                tabstop: 123,
                transform: None,
            }],
        );
        assert_snippet(
            "$farboo",
            &[Variable {
                name: "farboo".into(),
                transform: None,
                default: None,
            }],
        );
        assert_snippet(
            "$far12boo",
            &[Variable {
                name: "far12boo".into(),
                transform: None,
                default: None,
            }],
        );
        assert_snippet(
            "000_${far}_000",
            &[
                Text("000_".into()),
                Variable {
                    name: "far".into(),
                    transform: None,
                    default: None,
                },
                Text("_000".into()),
            ],
        );
    }

    #[test]
    fn parse_variable_transform() {
        assert_snippet(
            "${foo///}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: Tendril::new(),
                    replacement: Vec::new(),
                    options: Tendril::new(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/regex/format/gmi}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: "regex".into(),
                    replacement: vec![FormatItem::Text("format".into())],
                    options: "gmi".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/([A-Z][a-z])/format/}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: "([A-Z][a-z])".into(),
                    replacement: vec![FormatItem::Text("format".into())],
                    options: Tendril::new(),
                }),
                default: None,
            }],
        );

        // invalid regex TODO: reneable tests once we actually parse this regex flavor
        // assert_text(
        //     "${foo/([A-Z][a-z])/format/GMI}",
        //     "${foo/([A-Z][a-z])/format/GMI}",
        // );
        // assert_text(
        //     "${foo/([A-Z][a-z])/format/funky}",
        //     "${foo/([A-Z][a-z])/format/funky}",
        // );
        // assert_text("${foo/([A-Z][a-z]/format/}", "${foo/([A-Z][a-z]/format/}");
        assert_text(
            "${foo/regex\\/format/options}",
            "${foo/regex\\/format/options}",
        );

        // tricky regex
        assert_snippet(
            "${foo/m\\/atch/$1/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: "m/atch".into(),
                    replacement: vec![FormatItem::Capture(1)],
                    options: "i".into(),
                }),
                default: None,
            }],
        );

        // incomplete
        assert_text("${foo///", "${foo///");
        assert_text("${foo/regex/format/options", "${foo/regex/format/options");

        // format string
        assert_snippet(
            "${foo/.*/${0:fooo}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![FormatItem::Conditional(0, Tendril::new(), "fooo".into())],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/${1}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![FormatItem::Capture(1)],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/$1/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![FormatItem::Capture(1)],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/This-$1-encloses/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![
                        FormatItem::Text("This-".into()),
                        FormatItem::Capture(1),
                        FormatItem::Text("-encloses".into()),
                    ],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/complex${1:else}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![
                        FormatItem::Text("complex".into()),
                        FormatItem::Conditional(1, Tendril::new(), "else".into()),
                    ],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/complex${1:-else}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![
                        FormatItem::Text("complex".into()),
                        FormatItem::Conditional(1, Tendril::new(), "else".into()),
                    ],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/complex${1:+if}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![
                        FormatItem::Text("complex".into()),
                        FormatItem::Conditional(1, "if".into(), Tendril::new()),
                    ],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/complex${1:?if:else}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![
                        FormatItem::Text("complex".into()),
                        FormatItem::Conditional(1, "if".into(), "else".into()),
                    ],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${foo/.*/complex${1:/upcase}/i}",
            &[Variable {
                name: "foo".into(),
                transform: Some(Transform {
                    regex: ".*".into(),
                    replacement: vec![
                        FormatItem::Text("complex".into()),
                        FormatItem::CaseChange(1, CaseChange::Upcase),
                    ],
                    options: "i".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${TM_DIRECTORY/src\\//$1/}",
            &[Variable {
                name: "TM_DIRECTORY".into(),
                transform: Some(Transform {
                    regex: "src/".into(),
                    replacement: vec![FormatItem::Capture(1)],
                    options: Tendril::new(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${TM_SELECTED_TEXT/a/\\/$1/g}",
            &[Variable {
                name: "TM_SELECTED_TEXT".into(),
                transform: Some(Transform {
                    regex: "a".into(),
                    replacement: vec![FormatItem::Text("/".into()), FormatItem::Capture(1)],
                    options: "g".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${TM_SELECTED_TEXT/a/in\\/$1ner/g}",
            &[Variable {
                name: "TM_SELECTED_TEXT".into(),
                transform: Some(Transform {
                    regex: "a".into(),
                    replacement: vec![
                        FormatItem::Text("in/".into()),
                        FormatItem::Capture(1),
                        FormatItem::Text("ner".into()),
                    ],
                    options: "g".into(),
                }),
                default: None,
            }],
        );
        assert_snippet(
            "${TM_SELECTED_TEXT/a/end\\//g}",
            &[Variable {
                name: "TM_SELECTED_TEXT".into(),
                transform: Some(Transform {
                    regex: "a".into(),
                    replacement: vec![FormatItem::Text("end/".into())],
                    options: "g".into(),
                }),
                default: None,
            }],
        );
    }
    // TODO port more tests from https://github.com/microsoft/vscode/blob/dce493cb6e36346ef2714e82c42ce14fc461b15c/src/vs/editor/contrib/snippet/test/browser/snippetParser.test.ts
}
