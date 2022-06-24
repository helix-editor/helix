use std::borrow::Cow;

/// Get the vec of escaped / quoted / doublequoted filenames from the input str
pub fn shellwords(input: &str) -> Vec<Cow<'_, str>> {
    enum State {
        Normal,
        NormalEscaped,
        Quoted,
        QuoteEscaped,
        Dquoted,
        DquoteEscaped,
    }

    use State::*;

    let mut state = Normal;
    let mut args: Vec<Cow<str>> = Vec::new();
    let mut escaped = String::with_capacity(input.len());

    let mut start = 0;
    let mut end = 0;

    for (i, c) in input.char_indices() {
        state = match state {
            Normal => match c {
                '\\' => {
                    if cfg!(unix) {
                        escaped.push_str(&input[start..i]);
                        start = i + 1;
                        NormalEscaped
                    } else {
                        Normal
                    }
                }
                '"' => {
                    end = i;
                    Dquoted
                }
                '\'' => {
                    end = i;
                    Quoted
                }
                c if c.is_ascii_whitespace() => {
                    end = i;
                    Normal
                }
                _ => Normal,
            },
            NormalEscaped => Normal,
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
                    Normal
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
                    Normal
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
}
