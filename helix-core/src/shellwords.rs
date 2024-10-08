use std::borrow::Cow;

/// Auto escape for shellwords usage.
pub fn escape(input: Cow<str>) -> Cow<str> {
    if !input.chars().any(|x| x.is_ascii_whitespace()) {
        input
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

pub struct Shellwords<'a> {
    state: State,
    /// Shellwords where whitespace and escapes has been resolved.
    words: Vec<Cow<'a, str>>,
    /// The parts of the input that are divided into shellwords. This can be
    /// used to retrieve the original text for a given word by looking up the
    /// same index in the Vec as the word in `words`.
    parts: Vec<&'a str>,
}

impl<'a> From<&'a str> for Shellwords<'a> {
    fn from(input: &'a str) -> Self {
        use State::*;
        let mut state = Unquoted;
        let mut words = Vec::new();
        let mut parts = Vec::new();
        let mut escaped = String::with_capacity(input.len());
        let mut inside_variable_expansion = false;
        let mut nested_variable_expansion_count = 0;
        let mut part_start = 0;
        let mut unescaped_start = 0;
        let mut end = 0;

        for (i, c) in input.char_indices() {
            if c == '%' {
                //%sh{this "should" be escaped}
                if let Some(t) = input.get(i + 1..i + 3) {
                    if t == "sh" {
                        nested_variable_expansion_count += 1;
                        inside_variable_expansion = true;
                    }
                }
                //%{this "should" be escaped}
                if let Some(t) = input.get(i + 1..i + 2) {
                    if t == "{" {
                        nested_variable_expansion_count += 1;
                        inside_variable_expansion = true;
                    }
                }
            }
            if c == '}' {
                nested_variable_expansion_count -= 1;
                if nested_variable_expansion_count == 0 {
                    inside_variable_expansion = false;
                }
            }

            state = if !inside_variable_expansion {
                match state {
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
                                escaped.push_str(&input[unescaped_start..i]);
                                unescaped_start = i + 1;
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
                                escaped.push_str(&input[unescaped_start..i]);
                                unescaped_start = i + 1;
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
                                escaped.push_str(&input[unescaped_start..i]);
                                unescaped_start = i + 1;
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
                                escaped.push_str(&input[unescaped_start..i]);
                                unescaped_start = i + 1;
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
                }
            } else {
                state
            };

            let c_len = c.len_utf8();
            if i == input.len() - c_len && end == 0 {
                end = i + c_len;
            }

            if end > 0 {
                let esc_trim = escaped.trim();
                let inp = &input[unescaped_start..end];

                if !(esc_trim.is_empty() && inp.trim().is_empty()) {
                    if esc_trim.is_empty() {
                        words.push(inp.into());
                        parts.push(inp);
                    } else {
                        words.push([escaped, inp.into()].concat().into());
                        parts.push(&input[part_start..end]);
                        escaped = "".to_string();
                    }
                }
                unescaped_start = i + 1;
                part_start = i + 1;
                end = 0;
            }
        }

        debug_assert!(words.len() == parts.len());

        Self {
            state,
            words,
            parts,
        }
    }
}

impl<'a> Shellwords<'a> {
    /// Checks that the input ends with a whitespace character which is not escaped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use helix_core::shellwords::Shellwords;
    /// assert_eq!(Shellwords::from(" ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open foo.txt ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open").ends_with_whitespace(), false);
    /// #[cfg(unix)]
    /// assert_eq!(Shellwords::from(":open a\\ ").ends_with_whitespace(), false);
    /// #[cfg(unix)]
    /// assert_eq!(Shellwords::from(":open a\\ b.txt").ends_with_whitespace(), false);
    /// ```
    pub fn ends_with_whitespace(&self) -> bool {
        matches!(self.state, State::OnWhitespace)
    }

    /// Returns the list of shellwords calculated from the input string.
    pub fn words(&self) -> &[Cow<'a, str>] {
        &self.words
    }

    /// Returns a list of strings which correspond to [`Self::words`] but represent the original
    /// text in the input string - including escape characters - without separating whitespace.
    pub fn parts(&self) -> &[&'a str] {
        &self.parts
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn test_normal() {
        let input = r#":o single_word twó wörds \three\ \"with\ escaping\\"#;
        let shellwords = Shellwords::from(input);
        let result = shellwords.words().to_vec();
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
        let shellwords = Shellwords::from(input);
        let result = shellwords.words().to_vec();
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
    fn test_expansion() {
        let input = r#"echo %{filename} %{linenumber}"#;
        let shellwords = Shellwords::from(input);
        let result = shellwords.words().to_vec();
        let expected = vec![
            Cow::from("echo"),
            Cow::from("%{filename}"),
            Cow::from("%{linenumber}"),
        ];
        assert_eq!(expected, result);

        let input = r#"echo %{filename} 'world' %{something to 'escape}"#;
        let shellwords = Shellwords::from(input);
        let result = shellwords.words().to_vec();
        let expected = vec![
            Cow::from("echo"),
            Cow::from("%{filename}"),
            Cow::from("world"),
            Cow::from("%{something to 'escape}"),
        ];
        assert_eq!(expected, result);
        let input = r#"echo %sh{%sh{%{filename}}} cool"#;
        let shellwords = Shellwords::from(input);
        let result = shellwords.words().to_vec();
        let expected = vec![
            Cow::from("echo"),
            Cow::from("%sh{%sh{%{filename}}}"),
            Cow::from("cool"),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_quoted() {
        let quoted =
            r#":o 'single_word' 'twó wörds' '' ' ''\three\' \"with\ escaping\\' 'quote incomplete"#;
        let shellwords = Shellwords::from(quoted);
        let result = shellwords.words().to_vec();
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
        let shellwords = Shellwords::from(dquoted);
        let result = shellwords.words().to_vec();
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
        let shellwords = Shellwords::from(dquoted);
        let result = shellwords.words().to_vec();
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
            r#":set statusline.center ["file-type","file-encoding"] '["list", "in", "quotes"]'"#;
        let shellwords = Shellwords::from(input);
        let result = shellwords.words().to_vec();
        let expected = vec![
            Cow::from(":set"),
            Cow::from("statusline.center"),
            Cow::from(r#"["file-type","file-encoding"]"#),
            Cow::from(r#"["list", "in", "quotes"]"#),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    #[cfg(unix)]
    fn test_escaping_unix() {
        assert_eq!(escape("foobar".into()), Cow::Borrowed("foobar"));
        assert_eq!(escape("foo bar".into()), Cow::Borrowed("foo\\ bar"));
        assert_eq!(escape("foo\tbar".into()), Cow::Borrowed("foo\\\tbar"));
    }

    #[test]
    #[cfg(windows)]
    fn test_escaping_windows() {
        assert_eq!(escape("foobar".into()), Cow::Borrowed("foobar"));
        assert_eq!(escape("foo bar".into()), Cow::Borrowed("\"foo bar\""));
    }

    #[test]
    #[cfg(unix)]
    fn test_parts() {
        assert_eq!(Shellwords::from(":o a").parts(), &[":o", "a"]);
        assert_eq!(Shellwords::from(":o a\\ ").parts(), &[":o", "a\\ "]);
    }

    #[test]
    #[cfg(windows)]
    fn test_parts() {
        assert_eq!(Shellwords::from(":o a").parts(), &[":o", "a"]);
        assert_eq!(Shellwords::from(":o a\\ ").parts(), &[":o", "a\\"]);
    }

    #[test]
    fn test_multibyte_at_end() {
        assert_eq!(Shellwords::from("𒀀").parts(), &["𒀀"]);
        assert_eq!(
            Shellwords::from(":sh echo 𒀀").parts(),
            &[":sh", "echo", "𒀀"]
        );
        assert_eq!(
            Shellwords::from(":sh echo 𒀀 hello world𒀀").parts(),
            &[":sh", "echo", "𒀀", "hello", "world𒀀"]
        );
    }
}
