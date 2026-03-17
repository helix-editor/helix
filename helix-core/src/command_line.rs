//! Types and parsing code for command mode (`:`) input.
//!
//! Command line parsing is done in steps:
//!
//! * The `Tokenizer` iterator returns `Token`s from the command line input naively - without
//!   accounting for a command's signature.
//! * When executing a command (pressing `<ret>` in command mode), tokens are expanded with
//!   information from the editor like the current cursor line or column. Otherwise the tokens
//!   are unwrapped to their inner content.
//! * `Args` interprets the contents (potentially expanded) as flags or positional arguments.
//!   When executing a command, `Args` performs validations like checking the number of positional
//!   arguments supplied and whether duplicate or unknown flags were supplied.
//!
//! `Args` is the interface used by typable command implementations. `Args` may be treated as a
//! slice of `Cow<str>` or `&str` to access positional arguments, for example `for arg in args`
//! iterates over positional args (never flags) and `&args[0]` always corresponds to the first
//! positional. Use `Args::has_flag` and `Args::get_flag` to read any specified flags.
//!
//! `Args` and `Tokenizer` are intertwined. `Args` may ask the `Tokenizer` for the rest of the
//! command line as a single token after the configured number of positionals has been reached
//! (according to `raw_after`). This is used for the custom parsing in `:set-option` and
//! `:toggle-option` for example. Outside of executing commands, the `Tokenizer` can be used
//! directly to interpret a string according to the regular tokenization rules.

use std::{borrow::Cow, error::Error, fmt};

/// Splits a command line into the command and arguments parts.
///
/// The third tuple member describes whether the command part is finished. When this boolean is
/// true the completion code for the command line should complete command names, otherwise
/// command arguments.
pub fn split(line: &str) -> (&str, &str, bool) {
    const SEPARATOR_PATTERN: [char; 2] = [' ', '\t'];

    let (command, rest) = line.split_once(SEPARATOR_PATTERN).unwrap_or((line, ""));

    let complete_command =
        command.is_empty() || (rest.trim().is_empty() && !line.ends_with(SEPARATOR_PATTERN));

    (command, rest, complete_command)
}

/// The kind of expansion to use on the token's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionKind {
    /// Expand variables from the editor's state.
    ///
    /// For example `%{cursor_line}`.
    Variable,
    /// Treat the token contents as hexadecimal corresponding to a Unicode codepoint value.
    ///
    /// For example `%u{25CF}`.
    Unicode,
    /// Run the token's contents via the configured shell program.
    ///
    /// For example `%sh{echo hello}`.
    Shell,
}

impl ExpansionKind {
    pub const VARIANTS: &'static [Self] = &[Self::Variable, Self::Unicode, Self::Shell];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Variable => "",
            Self::Unicode => "u",
            Self::Shell => "sh",
        }
    }

    pub fn from_kind(name: &str) -> Option<Self> {
        match name {
            "" => Some(Self::Variable),
            "u" => Some(Self::Unicode),
            "sh" => Some(Self::Shell),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    Single,
    Backtick,
}

impl Quote {
    pub const fn char(&self) -> char {
        match self {
            Self::Single => '\'',
            Self::Backtick => '`',
        }
    }

    // Quotes can be escaped by doubling them: `'hello '' world'` becomes `hello ' world`.
    pub const fn escape(&self) -> &'static str {
        match self {
            Self::Single => "''",
            Self::Backtick => "``",
        }
    }
}

/// The type of argument being written.
///
/// The token kind decides how an argument in the command line will be expanded upon hitting
/// `<ret>` in command mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// Unquoted text.
    ///
    /// For example in `:echo hello world`, "hello" and "world" are raw tokens.
    Unquoted,
    /// Quoted text which is interpreted literally.
    ///
    /// The purpose of this kind is to avoid splitting arguments on whitespace. For example
    /// `:open 'a b.txt'` will result in opening a file with a single argument `"a b.txt"`.
    ///
    /// Using expansions within single quotes or backticks will result in the expansion text
    /// being shown literally. For example `:echo '%u{0020}'` will print `"%u{0020}"` to the
    /// statusline.
    Quoted(Quote),
    /// Text within double quote delimiters (`"`).
    ///
    /// The inner text of a double quoted argument can be further expanded. For example
    /// `:echo "line: #%{cursor_line}"` could print `"line: #1"` to the statusline.
    Expand,
    /// An expansion / "percent token".
    ///
    /// These take the form `%[<kind>]<open><contents><close>`. See `ExpansionKind`.
    Expansion(ExpansionKind),
    /// A token kind that exists for the sake of completion.
    ///
    /// In input like `%foo` this token contains the text `"%foo"`. The content start is the byte
    /// after the percent token.
    ///
    /// When `Tokenizer` is passed `true` for its `validate` parameter this token cannot be
    /// returned: inputs that would return this token get a validation error instead.
    ExpansionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    /// The byte index into the input where the token's content starts.
    ///
    /// For quoted text this means the byte after the quote. For expansions this means the byte
    /// after the opening delimiter.
    pub content_start: usize,
    /// The inner content of the token.
    ///
    /// Usually this content borrows from the input but an owned value may be used in cases of
    /// escaping. On Unix systems a raw token like `a\ b` has the contents `"a b"`.
    pub content: Cow<'a, str>,
    /// Whether the token's opening delimiter is closed.
    ///
    /// For example a quote `"foo"` is closed but not `"foo` or an expansion `%sh{..}` is closed
    /// but not `%sh{echo {}`.
    pub is_terminated: bool,
}

impl<'a> Token<'a> {
    pub fn empty_at(content_start: usize) -> Self {
        Self {
            kind: TokenKind::Unquoted,
            content_start,
            content: Cow::Borrowed(""),
            is_terminated: false,
        }
    }

    pub fn expand(content: impl Into<Cow<'a, str>>) -> Self {
        Self {
            kind: TokenKind::Expand,
            content_start: 0,
            content: content.into(),
            is_terminated: true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenizeError<'a> {
    UnterminatedToken { token: Token<'a> },
    MissingExpansionDelimiter { expansion: &'a str },
    UnknownExpansion { kind: &'a str },
}

impl fmt::Display for TokenizeError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedToken { token } => {
                write!(f, "unterminated token {}", token.content)
            }
            Self::MissingExpansionDelimiter { expansion } => {
                if expansion.is_empty() {
                    write!(f, "'%' was not properly escaped. Please use '%%'")
                } else {
                    write!(f, "missing a string delimiter after '%{expansion}'")
                }
            }
            Self::UnknownExpansion { kind } => {
                write!(f, "unknown expansion '{kind}'")
            }
        }
    }
}

impl Error for TokenizeError<'_> {}

#[derive(Debug)]
pub struct Tokenizer<'a> {
    input: &'a str,
    /// Whether to return errors in the iterator for failed validations like unterminated strings
    /// or expansions. When this is set to `false` the iterator will never return `Err`.
    validate: bool,
    /// The current byte index of the input being considered.
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str, validate: bool) -> Self {
        Self {
            input,
            validate,
            pos: 0,
        }
    }

    /// Returns the current byte index position of the parser in the input.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the rest of the input as a single `TokenKind::Expand` token literally.
    ///
    /// Returns `None` if the tokenizer is already at the end of the input or advances the
    /// tokenizer to the end of the input otherwise. Leading whitespace characters are skipped.
    /// Quoting is not interpreted.
    pub fn rest(&mut self) -> Option<Token<'a>> {
        self.skip_blanks();

        if self.pos == self.input.len() {
            return None;
        }

        let content_start = self.pos;
        self.pos = self.input.len();
        Some(Token {
            kind: TokenKind::Expand,
            content_start,
            content: Cow::Borrowed(&self.input[content_start..]),
            is_terminated: false,
        })
    }

    fn byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos + 1).copied()
    }

    fn prev_byte(&self) -> Option<u8> {
        self.pos
            .checked_sub(1)
            .map(|idx| self.input.as_bytes()[idx])
    }

    fn skip_blanks(&mut self) {
        while let Some(b' ' | b'\t') = self.byte() {
            self.pos += 1;
        }
    }

    fn parse_unquoted(&mut self) -> Cow<'a, str> {
        // Note that `String::new` starts with no allocation. We only allocate if we see a
        // backslash escape (on Unix only).
        let mut escaped = String::new();
        let mut start = self.pos;

        while let Some(byte) = self.byte() {
            if matches!(byte, b' ' | b'\t') {
                if cfg!(unix) && self.prev_byte() == Some(b'\\') {
                    // Push everything up to but not including the backslash and then this
                    // whitespace character.
                    escaped.push_str(&self.input[start..self.pos - 1]);
                    escaped.push(byte as char);
                    start = self.pos + 1;
                } else if escaped.is_empty() {
                    return Cow::Borrowed(&self.input[start..self.pos]);
                } else {
                    break;
                }
            }

            self.pos += 1;
        }

        // Special case for a trailing backslash on Unix: exclude the backslash from the content.
        // This improves the behavior of completions like `":open a\\"` (trailing backslash).
        let end = if cfg!(unix) && self.prev_byte() == Some(b'\\') {
            self.pos - 1
        } else {
            self.pos
        };

        if escaped.is_empty() {
            assert_eq!(self.pos, self.input.len());
            Cow::Borrowed(&self.input[start..end])
        } else {
            escaped.push_str(&self.input[start..end]);
            Cow::Owned(escaped)
        }
    }

    /// Parses a string quoted by the given grapheme cluster.
    ///
    /// The position of the tokenizer is asserted to be immediately after the quote grapheme
    /// cluster.
    fn parse_quoted(&mut self, quote: u8) -> (Cow<'a, str>, bool) {
        assert_eq!(self.byte(), Some(quote));
        self.pos += 1;

        let mut escaped = String::new();
        while let Some(offset) = self.input[self.pos..].find(quote as char) {
            let idx = self.pos + offset;
            if self.input.as_bytes().get(idx + 1) == Some(&quote) {
                // Treat two quotes in a row as an escape.
                escaped.push_str(&self.input[self.pos..idx + 1]);
                // Advance past the escaped quote.
                self.pos = idx + 2;
            } else {
                // Otherwise this quote string is finished.
                let quoted = if escaped.is_empty() {
                    Cow::Borrowed(&self.input[self.pos..idx])
                } else {
                    escaped.push_str(&self.input[self.pos..idx]);
                    Cow::Owned(escaped)
                };
                // Advance past the closing quote.
                self.pos = idx + 1;
                return (quoted, true);
            }
        }

        let quoted = if escaped.is_empty() {
            Cow::Borrowed(&self.input[self.pos..])
        } else {
            escaped.push_str(&self.input[self.pos..]);
            Cow::Owned(escaped)
        };
        self.pos = self.input.len();

        (quoted, false)
    }

    /// Parses the percent token expansion under the tokenizer's cursor.
    ///
    /// This function should only be called when the tokenizer's cursor is on a non-escaped
    /// percent token.
    pub fn parse_percent_token(&mut self) -> Option<Result<Token<'a>, TokenizeError<'a>>> {
        assert_eq!(self.byte(), Some(b'%'));

        self.pos += 1;
        let kind_start = self.pos;
        self.pos += self.input[self.pos..]
            .bytes()
            .take_while(|b| b.is_ascii_lowercase())
            .count();
        let kind = &self.input[kind_start..self.pos];

        let (open, close) = match self.byte() {
            // We support a couple of hard-coded chars only to make sure we can provide more
            // useful errors and avoid weird behavior in case of typos. These should cover
            // practical cases.
            Some(b'(') => (b'(', b')'),
            Some(b'[') => (b'[', b']'),
            Some(b'{') => (b'{', b'}'),
            Some(b'<') => (b'<', b'>'),
            Some(b'\'') => (b'\'', b'\''),
            Some(b'\"') => (b'\"', b'\"'),
            Some(b'|') => (b'|', b'|'),
            Some(_) | None => {
                return Some(if self.validate {
                    Err(TokenizeError::MissingExpansionDelimiter { expansion: kind })
                } else {
                    Ok(Token {
                        kind: TokenKind::ExpansionKind,
                        content_start: kind_start,
                        content: Cow::Borrowed(kind),
                        is_terminated: false,
                    })
                });
            }
        };
        // The content start for expansions is the start of the content - after the opening
        // delimiter grapheme.
        let content_start = self.pos + 1;
        let kind = match ExpansionKind::from_kind(kind) {
            Some(kind) => TokenKind::Expansion(kind),
            None if self.validate => {
                return Some(Err(TokenizeError::UnknownExpansion { kind }));
            }
            None => TokenKind::Expand,
        };

        let (content, is_terminated) = if open == close {
            self.parse_quoted(open)
        } else {
            self.parse_quoted_balanced(open, close)
        };

        let token = Token {
            kind,
            content_start,
            content,
            is_terminated,
        };

        if self.validate && !is_terminated {
            return Some(Err(TokenizeError::UnterminatedToken { token }));
        }

        Some(Ok(token))
    }

    /// Parse the next string under the cursor given an open and closing pair.
    ///
    /// The open and closing pair are different ASCII characters. The cursor is asserted to be
    /// immediately after the opening delimiter.
    ///
    /// This function parses with nesting support. `%sh{echo {hello}}` for example should consume
    /// the entire input and not quit after the first '}' character is found.
    fn parse_quoted_balanced(&mut self, open: u8, close: u8) -> (Cow<'a, str>, bool) {
        assert_eq!(self.byte(), Some(open));
        self.pos += 1;
        let start = self.pos;
        let mut level = 1;

        while let Some(offset) = self.input[self.pos..].find([open as char, close as char]) {
            let idx = self.pos + offset;
            // Move past the delimiter.
            self.pos = idx + 1;

            let byte = self.input.as_bytes()[idx];
            if byte == open {
                level += 1;
            } else if byte == close {
                level -= 1;
                if level == 0 {
                    break;
                }
            } else {
                unreachable!()
            }
        }

        let is_terminated = level == 0;
        let end = if is_terminated {
            // Exclude the closing delimiter from the token's content.
            self.pos - 1
        } else {
            // When the token is not closed, advance to the end of the input.
            self.pos = self.input.len();
            self.pos
        };

        (Cow::Borrowed(&self.input[start..end]), is_terminated)
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token<'a>, TokenizeError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_blanks();

        let byte = self.byte()?;
        match byte {
            b'"' | b'\'' | b'`' => {
                let content_start = self.pos + 1;
                let (content, is_terminated) = self.parse_quoted(byte);
                let token = Token {
                    kind: match byte {
                        b'"' => TokenKind::Expand,
                        b'\'' => TokenKind::Quoted(Quote::Single),
                        b'`' => TokenKind::Quoted(Quote::Backtick),
                        _ => unreachable!(),
                    },
                    content_start,
                    content,
                    is_terminated,
                };

                Some(if self.validate && !is_terminated {
                    Err(TokenizeError::UnterminatedToken { token })
                } else {
                    Ok(token)
                })
            }
            b'%' => self.parse_percent_token(),
            _ => {
                let content_start = self.pos;

                // Allow backslash escaping on Unix for quotes or expansions
                if cfg!(unix)
                    && byte == b'\\'
                    && matches!(self.peek_byte(), Some(b'"' | b'\'' | b'`' | b'%'))
                {
                    self.pos += 1;
                }

                Some(Ok(Token {
                    kind: TokenKind::Unquoted,
                    content_start,
                    content: self.parse_unquoted(),
                    is_terminated: false,
                }))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[track_caller]
    fn assert_tokens(input: &str, expected: &[&str]) {
        let actual: Vec<_> = Tokenizer::new(input, true)
            .map(|arg| arg.unwrap().content)
            .collect();
        let actual: Vec<_> = actual.iter().map(|c| c.as_ref()).collect();

        assert_eq!(actual.as_slice(), expected);
    }

    #[track_caller]
    fn assert_incomplete_tokens(input: &str, expected: &[&str]) {
        assert!(
            Tokenizer::new(input, true).collect::<Result<Vec<_>, _>>().is_err(),
            "`assert_incomplete_tokens` only accepts input that fails validation, consider using `assert_tokens` instead"
        );
        let actual: Vec<_> = Tokenizer::new(input, false)
            .map(|arg| arg.unwrap().content)
            .collect();
        let actual: Vec<_> = actual.iter().map(|c| c.as_ref()).collect();

        assert_eq!(actual.as_slice(), expected);
    }

    #[test]
    fn tokenize_unquoted() {
        assert_tokens("", &[]);
        assert_tokens("hello", &["hello"]);
        assert_tokens("hello world", &["hello", "world"]);
        // Any amount of whitespace is considered a separator.
        assert_tokens("hello\t \tworld", &["hello", "world"]);
    }

    // This escaping behavior is specific to Unix systems.
    #[cfg(unix)]
    #[test]
    fn tokenize_backslash_unix() {
        assert_tokens(r#"hello\ world"#, &["hello world"]);
        assert_tokens(r#"one\ two three"#, &["one two", "three"]);
        assert_tokens(r#"one two\ three"#, &["one", "two three"]);
        // Trailing backslash is ignored - this improves completions.
        assert_tokens(r#"hello\"#, &["hello"]);
        // The backslash at the start of the double quote makes the quote be treated as raw.
        // For the backslash before the ending quote the token is already considered raw so the
        // backslash and quote are treated literally.
        assert_tokens(
            r#"echo \"hello        world\""#,
            &["echo", r#""hello"#, r#"world\""#],
        );
    }

    #[test]
    fn tokenize_backslash() {
        assert_tokens(r#"\n"#, &["\\n"]);
        assert_tokens(r#"'\'"#, &["\\"]);
    }

    #[test]
    fn tokenize_quoting() {
        // Using a quote character twice escapes it.
        assert_tokens(r#"''"#, &[""]);
        assert_tokens(r#""""#, &[""]);
        assert_tokens(r#"``"#, &[""]);
        assert_tokens(r#"echo """#, &["echo", ""]);

        assert_tokens(r#"'hello'"#, &["hello"]);
        assert_tokens(r#"'hello world'"#, &["hello world"]);

        assert_tokens(r#""hello "" world""#, &["hello \" world"]);
    }

    #[test]
    fn tokenize_percent() {
        // Pair delimiters:
        assert_tokens(r#"echo %{hello world}"#, &["echo", "hello world"]);
        assert_tokens(r#"echo %[hello world]"#, &["echo", "hello world"]);
        assert_tokens(r#"echo %(hello world)"#, &["echo", "hello world"]);
        assert_tokens(r#"echo %<hello world>"#, &["echo", "hello world"]);
        assert_tokens(r#"echo %|hello world|"#, &["echo", "hello world"]);
        assert_tokens(r#"echo %'hello world'"#, &["echo", "hello world"]);
        assert_tokens(r#"echo %"hello world""#, &["echo", "hello world"]);
        // When invoking a command, double percents can be used within a string as an escape for
        // the percent. This is done in the expansion code though, not in the parser here.
        assert_tokens(r#"echo "%%hello world""#, &["echo", "%%hello world"]);
        // Different kinds of quotes nested:
        assert_tokens(
            r#"echo "%sh{echo 'hello world'}""#,
            &["echo", r#"%sh{echo 'hello world'}"#],
        );
        // Nesting of the expansion delimiter:
        assert_tokens(r#"echo %{hello {x} world}"#, &["echo", "hello {x} world"]);
        assert_tokens(
            r#"echo %{hello {{😎}} world}"#,
            &["echo", "hello {{😎}} world"],
        );

        // Balanced nesting:
        assert_tokens(
            r#"echo %{hello {}} world}"#,
            &["echo", "hello {}", "world}"],
        );

        // Recursive expansions:
        assert_tokens(
            r#"echo %sh{echo "%{cursor_line}"}"#,
            &["echo", r#"echo "%{cursor_line}""#],
        );
        // Completion should provide variable names here. (Unbalanced nesting)
        assert_incomplete_tokens(r#"echo %sh{echo "%{c"#, &["echo", r#"echo "%{c"#]);
        assert_incomplete_tokens(r#"echo %{hello {{} world}"#, &["echo", "hello {{} world}"]);
    }
}
