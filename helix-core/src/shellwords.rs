use std::borrow::Cow;

/// Auto escape for shellwords usage.
pub fn escape(input: &str) -> Cow<'_, str> {
    if !input.chars().any(|x| x.is_ascii_whitespace()) {
        Cow::Borrowed(input)
    } else if cfg!(unix) {
        Cow::Owned(input.chars().fold(String::new(), |mut buf, c| {
            if c.is_ascii_whitespace() {
                buf.push('\\');
            }
            buf.push(c);
            buf
        }))
    } else {
        Cow::Owned(format!("\"{}\"", input))
    }
}

enum State {
    OnWhitespace,
    Unquoted,
    UnquotedEscaped,
    Quoted,
    QuoteEscaped,
    Dquoted,
    DquoteEscaped,
}

/// Get the vec of escaped / quoted / doublequoted filenames from the input str
pub fn shellwords(input: &str) -> Vec<Cow<'_, str>> {
    use State::*;

    let mut state = Unquoted;
    let mut args: Vec<Cow<str>> = Vec::new();
    let mut escaped = String::with_capacity(input.len());

    let mut start = 0;
    let mut end = 0;

    for (i, c) in input.char_indices() {
        state = match state {
            OnWhitespace => match c {
                '"' => {
                    end = i;
                    Dquoted
                }
                '\'' => {
                    end = i;
                    Quoted
                }
                '\\' => {
                    if cfg!(unix) {
                        escaped.push_str(&input[start..i]);
                        start = i + 1;
                        UnquotedEscaped
                    } else {
                        OnWhitespace
                    }
                }
                c if c.is_ascii_whitespace() => {
                    end = i;
                    OnWhitespace
                }
                _ => Unquoted,
            },
            Unquoted => match c {
                '\\' => {
                    if cfg!(unix) {
                        escaped.push_str(&input[start..i]);
                        start = i + 1;
                        UnquotedEscaped
                    } else {
                        Unquoted
                    }
                }
                c if c.is_ascii_whitespace() => {
                    end = i;
                    OnWhitespace
                }
                _ => Unquoted,
            },
            UnquotedEscaped => Unquoted,
            Quoted => match c {
                '\\' => {
                    if cfg!(unix) {
                        escaped.push_str(&input[start..i]);
                        start = i + 1;
                        QuoteEscaped
                    } else {
                        Quoted
                    }
                }
                '\'' => {
                    end = i;
                    OnWhitespace
                }
                _ => Quoted,
            },
            QuoteEscaped => Quoted,
            Dquoted => match c {
                '\\' => {
                    if cfg!(unix) {
                        escaped.push_str(&input[start..i]);
                        start = i + 1;
                        DquoteEscaped
                    } else {
                        Dquoted
                    }
                }
                '"' => {
                    end = i;
                    OnWhitespace
                }
                _ => Dquoted,
            },
            DquoteEscaped => Dquoted,
        };

        if i >= input.len() - 1 && end == 0 {
            end = i + 1;
        }

        if end > 0 {
            let esc_trim = escaped.trim();
            let inp = &input[start..end];

            if !(esc_trim.is_empty() && inp.trim().is_empty()) {
                if esc_trim.is_empty() {
                    args.push(inp.into());
                } else {
                    args.push([escaped, inp.into()].concat().into());
                    escaped = "".to_string();
                }
            }
            start = i + 1;
            end = 0;
        }
    }
    args
}

/// Checks that the input ends with an ascii whitespace character which is
/// not escaped.
///
/// # Examples
///
/// ```rust
/// use helix_core::shellwords::ends_with_whitespace;
/// assert_eq!(ends_with_whitespace(" "), true);
/// assert_eq!(ends_with_whitespace(":open "), true);
/// assert_eq!(ends_with_whitespace(":open foo.txt "), true);
/// assert_eq!(ends_with_whitespace(":open"), false);
/// #[cfg(unix)]
/// assert_eq!(ends_with_whitespace(":open a\\ "), false);
/// #[cfg(unix)]
/// assert_eq!(ends_with_whitespace(":open a\\ b.txt"), false);
/// ```
pub fn ends_with_whitespace(input: &str) -> bool {
    use State::*;

    // Fast-lane: the input must end with a whitespace character
    // regardless of quoting.
    if !input.ends_with(|c: char| c.is_ascii_whitespace()) {
        return false;
    }

    let mut state = Unquoted;

    for c in input.chars() {
        state = match state {
            OnWhitespace => match c {
                '"' => Dquoted,
                '\'' => Quoted,
                '\\' if cfg!(unix) => UnquotedEscaped,
                '\\' => OnWhitespace,
                c if c.is_ascii_whitespace() => OnWhitespace,
                _ => Unquoted,
            },
            Unquoted => match c {
                '\\' if cfg!(unix) => UnquotedEscaped,
                '\\' => Unquoted,
                c if c.is_ascii_whitespace() => OnWhitespace,
                _ => Unquoted,
            },
            UnquotedEscaped => Unquoted,
            Quoted => match c {
                '\\' if cfg!(unix) => QuoteEscaped,
                '\\' => Quoted,
                '\'' => OnWhitespace,
                _ => Quoted,
            },
            QuoteEscaped => Quoted,
            Dquoted => match c {
                '\\' if cfg!(unix) => DquoteEscaped,
                '\\' => Dquoted,
                '"' => OnWhitespace,
                _ => Dquoted,
            },
            DquoteEscaped => Dquoted,
        }
    }

    matches!(state, OnWhitespace)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_normal() {
        let input = r#":o single_word twó wörds \three\ \"with\ escaping\\"#;
        let result = shellwords(input);
        let expected = vec![
            Cow::from(":o"),
            Cow::from("single_word"),
            Cow::from("twó"),
            Cow::from("wörds"),
            Cow::from("\\three\\"),
            Cow::from("\\"),
            Cow::from("with\\ escaping\\\\"),
        ];
        // TODO test is_owned and is_borrowed, once they get stabilized.
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_normal() {
        let input = r#":o single_word twó wörds \three\ \"with\ escaping\\"#;
        let result = shellwords(input);
        let expected = vec![
            Cow::from(":o"),
            Cow::from("single_word"),
            Cow::from("twó"),
            Cow::from("wörds"),
            Cow::from(r#"three "with escaping\"#),
        ];
        // TODO test is_owned and is_borrowed, once they get stabilized.
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_quoted() {
        let quoted =
            r#":o 'single_word' 'twó wörds' '' ' ''\three\' \"with\ escaping\\' 'quote incomplete"#;
        let result = shellwords(quoted);
        let expected = vec![
            Cow::from(":o"),
            Cow::from("single_word"),
            Cow::from("twó wörds"),
            Cow::from(r#"three' "with escaping\"#),
            Cow::from("quote incomplete"),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_dquoted() {
        let dquoted = r#":o "single_word" "twó wörds" "" "  ""\three\' \"with\ escaping\\" "dquote incomplete"#;
        let result = shellwords(dquoted);
        let expected = vec![
            Cow::from(":o"),
            Cow::from("single_word"),
            Cow::from("twó wörds"),
            Cow::from(r#"three' "with escaping\"#),
            Cow::from("dquote incomplete"),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_mixed() {
        let dquoted = r#":o single_word 'twó wörds' "\three\' \"with\ escaping\\""no space before"'and after' $#%^@ "%^&(%^" ')(*&^%''a\\\\\b' '"#;
        let result = shellwords(dquoted);
        let expected = vec![
            Cow::from(":o"),
            Cow::from("single_word"),
            Cow::from("twó wörds"),
            Cow::from("three' \"with escaping\\"),
            Cow::from("no space before"),
            Cow::from("and after"),
            Cow::from("$#%^@"),
            Cow::from("%^&(%^"),
            Cow::from(")(*&^%"),
            Cow::from(r#"a\\b"#),
            //last ' just changes to quoted but since we dont have anything after it, it should be ignored
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_lists() {
        let input =
            r#":set statusline.center ["file-type","file-encoding"] '["list", "in", "qoutes"]'"#;
        let result = shellwords(input);
        let expected = vec![
            Cow::from(":set"),
            Cow::from("statusline.center"),
            Cow::from(r#"["file-type","file-encoding"]"#),
            Cow::from(r#"["list", "in", "qoutes"]"#),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_escaping_unix() {
        assert_eq!(escape("foobar"), Cow::Borrowed("foobar"));
        assert_eq!(escape("foo bar"), Cow::Borrowed("foo\\ bar"));
        assert_eq!(escape("foo\tbar"), Cow::Borrowed("foo\\\tbar"));
    }

    #[test]
    #[cfg(windows)]
    fn test_escaping_windows() {
        assert_eq!(escape("foobar"), Cow::Borrowed("foobar"));
        assert_eq!(escape("foo bar"), Cow::Borrowed("\"foo bar\""));
    }
}
