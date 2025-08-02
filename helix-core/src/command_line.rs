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
//!
//! This module also defines structs for configuring the parsing of the command line for a
//! command. See `Flag` and `Signature`.

use std::{borrow::Cow, collections::HashMap, error::Error, fmt, ops, slice, vec};

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

/// A Unix-like flag that a command may accept.
///
/// For example the `:sort` command accepts a `--reverse` (or `-r` for shorthand) boolean flag
/// which controls the direction of sorting. Flags may accept an argument by setting the
/// `completions` field to `Some`.
#[derive(Debug, Clone, Copy)]
pub struct Flag {
    /// The name of the flag.
    ///
    /// This value is also used to construct the "longhand" version of the flag. For example a
    /// flag with a name "reverse" has a longhand `--reverse`.
    ///
    /// This value should be supplied when reading a flag out of the [Args] with [Args::get_flag]
    /// and [Args::has_flag]. The `:sort` command implementation for example should ask for
    /// `args.has_flag("reverse")`.
    pub name: &'static str,
    /// The character that can be used as a shorthand for the flag, optionally.
    ///
    /// For example a flag like "reverse" mentioned above might take an alias `Some('r')` to
    /// allow specifying the flag as `-r`.
    pub alias: Option<char>,
    pub doc: &'static str,
    /// The completion values to use when specifying an argument for a flag.
    ///
    /// This should be set to `None` for boolean flags and `Some(&["foo", "bar", "baz"])` for
    /// example for flags which accept options, with the strings corresponding to values that
    /// should be shown in completion.
    pub completions: Option<&'static [&'static str]>,
}

impl Flag {
    // This allows defining flags with the `..Flag::DEFAULT` shorthand. The `name` and `doc`
    // fields should always be overwritten.
    pub const DEFAULT: Self = Self {
        name: "",
        doc: "",
        alias: None,
        completions: None,
    };
}

/// A description of how a command's input should be handled.
///
/// Each typable command defines a signature (with the help of `Signature::DEFAULT`) at least to
/// declare how many positional arguments it accepts. Command flags are also declared in this
/// struct. The `raw_after` option may be set optionally to avoid evaluating quotes in parts of
/// the command line (useful for shell commands for example).
#[derive(Debug, Clone, Copy)]
#[allow(clippy::manual_non_exhaustive)]
pub struct Signature {
    /// The minimum and (optionally) maximum number of positional arguments a command may take.
    ///
    /// For example accepting exactly one positional can be specified with `(1, Some(1))` while
    /// accepting zero-or-more positionals can be specified as `(0, None)`.
    ///
    /// The number of positionals is checked when hitting `<ret>` in command mode. If the actual
    /// number of positionals is outside the declared range then the command is not executed and
    /// an error is shown instead. For example `:write` accepts zero or one positional arguments
    /// (`(0, Some(1))`). A command line like `:write a.txt b.txt` is outside the declared range
    /// and is not accepted.
    pub positionals: (usize, Option<usize>),
    /// The number of **positional** arguments for the parser to read with normal quoting rules.
    ///
    /// Once the number has been exceeded then the tokenizer returns the rest of the input as a
    /// `TokenKind::Expand` token (see `Tokenizer::rest`), meaning that quoting rules do not apply
    /// and none of the remaining text may be treated as a flag.
    ///
    /// If this is set to `None` then the entire command line is parsed with normal quoting and
    /// flag rules.
    ///
    /// A good example use-case for this option is `:toggle-option` which sets `Some(1)`.
    /// Everything up to the first positional argument is interpreted according to normal rules
    /// and the rest of the input is parsed "raw". This allows `:toggle-option` to perform custom
    /// parsing on the rest of the input - namely parsing complicated values as a JSON stream.
    /// `:toggle-option` could accept a flag in the future. If so, the flag would need to come
    /// before the first positional argument.
    ///
    /// Consider these lines for `:toggle-option` which sets `Some(1)`:
    ///
    /// * `:toggle foo` has one positional "foo" and no flags.
    /// * `:toggle foo bar` has two positionals. Expansions for `bar` are evaluated but quotes
    ///   and anything that looks like a flag are treated literally.
    /// * `:toggle foo --bar` has two positionals: `["foo", "--bar"]`. `--bar` is not considered
    ///   to be a flag because it comes after the first positional.
    /// * `:toggle --bar foo` has one positional "foo" and one flag "--bar".
    /// * `:toggle --bar foo --baz` has two positionals `["foo", "--baz"]` and one flag "--bar".
    pub raw_after: Option<u8>,
    /// A set of flags that a command may accept.
    ///
    /// See the `Flag` struct for more info.
    pub flags: &'static [Flag],
    /// Do not set this field. Use `..Signature::DEFAULT` to construct a `Signature` instead.
    // This field allows adding new fields later with minimal code changes. This works like a
    // `#[non_exhaustive]` annotation except that it supports the `..Signature::DEFAULT`
    // shorthand.
    pub _dummy: (),
}

impl Signature {
    // This allows defining signatures with the `..Signature::DEFAULT` shorthand. The
    // `positionals` field should always be overwritten.
    pub const DEFAULT: Self = Self {
        positionals: (0, None),
        raw_after: None,
        flags: &[],
        _dummy: (),
    };

    fn check_positional_count(&self, actual: usize) -> Result<(), ParseArgsError<'static>> {
        let (min, max) = self.positionals;
        if min <= actual && max.unwrap_or(usize::MAX) >= actual {
            Ok(())
        } else {
            Err(ParseArgsError::WrongPositionalCount { min, max, actual })
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseArgsError<'a> {
    WrongPositionalCount {
        min: usize,
        max: Option<usize>,
        actual: usize,
    },
    UnterminatedToken {
        token: Token<'a>,
    },
    DuplicatedFlag {
        flag: &'static str,
    },
    UnknownFlag {
        text: Cow<'a, str>,
    },
    FlagMissingArgument {
        flag: &'static str,
    },
    MissingExpansionDelimiter {
        expansion: &'a str,
    },
    UnknownExpansion {
        kind: &'a str,
    },
}

impl fmt::Display for ParseArgsError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPositionalCount { min, max, actual } => {
                write!(f, "expected ")?;
                let maybe_plural = |n| if n == 1 { "" } else { "s" };
                match (min, max) {
                    (0, Some(0)) => write!(f, "no arguments")?,
                    (min, Some(max)) if min == max => {
                        write!(f, "exactly {min} argument{}", maybe_plural(*min))?
                    }
                    (min, _) if actual < min => {
                        write!(f, "at least {min} argument{}", maybe_plural(*min))?
                    }
                    (_, Some(max)) if actual > max => {
                        write!(f, "at most {max} argument{}", maybe_plural(*max))?
                    }
                    // `actual` must be either less than `min` or greater than `max` for this type
                    // to be constructed.
                    _ => unreachable!(),
                }

                write!(f, ", got {actual}")
            }
            Self::UnterminatedToken { token } => {
                write!(f, "unterminated token {}", token.content)
            }
            Self::DuplicatedFlag { flag } => {
                write!(f, "flag '--{flag}' specified more than once")
            }
            Self::UnknownFlag { text } => write!(f, "unknown flag '{text}'"),
            Self::FlagMissingArgument { flag } => {
                write!(f, "flag '--{flag}' missing an argument")
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

impl Error for ParseArgsError<'_> {}

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
    pub fn parse_percent_token(&mut self) -> Option<Result<Token<'a>, ParseArgsError<'a>>> {
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
                    Err(ParseArgsError::MissingExpansionDelimiter { expansion: kind })
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
                return Some(Err(ParseArgsError::UnknownExpansion { kind }));
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
            return Some(Err(ParseArgsError::UnterminatedToken { token }));
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
    type Item = Result<Token<'a>, ParseArgsError<'a>>;

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
                    Err(ParseArgsError::UnterminatedToken { token })
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

#[derive(Debug, Default, Clone, Copy)]
pub enum CompletionState {
    #[default]
    Positional,
    Flag(Option<Flag>),
    FlagArgument(Flag),
}

/// A set of arguments provided to a command on the command line.
///
/// Regular arguments are called "positional" arguments (or "positionals" for short). Command line
/// input might also specify "flags" which can modify a command's behavior.
///
/// ```rust,ignore
/// // Say that the command accepts a "bar" flag which doesn't accept an argument itself.
/// // This input has two positionals, "foo" and "baz" and one flag "--bar".
/// let args = Args::parse("foo --bar baz", /* .. */);
/// // `Args` may be treated like a slice to access positionals.
/// assert_eq!(args.len(), 2);
/// assert_eq!(&args[0], "foo");
/// assert_eq!(&args[1], "baz");
/// // Use `has_flag` or `get_flag` to access flags.
/// assert!(args.has_flag("bar"));
/// ```
///
/// The `Args` type can be treated mostly the same as a slice when accessing positional arguments.
/// Common slice methods like `len`, `get`, `first` and `join` only expose positional arguments.
/// Additionally, common syntax like `for arg in args` or `&args[idx]` is supported for accessing
/// positional arguments.
///
/// To look up flags, use `Args::get_flag` for flags which should accept an argument or
/// `Args::has_flag` for boolean flags.
///
/// The way that `Args` is parsed from the input depends on a command's `Signature`. See the
/// `Signature` type for more details.
#[derive(Debug)]
pub struct Args<'a> {
    signature: Signature,
    /// Whether to validate the arguments.
    /// See the `ParseArgsError` type for the validations.
    validate: bool,
    /// Whether args pushed with `Self::push` should be treated as positionals even if they
    /// start with '-'.
    only_positionals: bool,
    state: CompletionState,
    positionals: Vec<Cow<'a, str>>,
    flags: HashMap<&'static str, Cow<'a, str>>,
}

impl Default for Args<'_> {
    fn default() -> Self {
        Self {
            signature: Signature::DEFAULT,
            validate: Default::default(),
            only_positionals: Default::default(),
            state: CompletionState::default(),
            positionals: Default::default(),
            flags: Default::default(),
        }
    }
}

impl<'a> Args<'a> {
    pub fn new(signature: Signature, validate: bool) -> Self {
        Self {
            signature,
            validate,
            only_positionals: false,
            positionals: Vec::new(),
            flags: HashMap::new(),
            state: CompletionState::default(),
        }
    }

    pub fn raw(positionals: Vec<Cow<'a, str>>) -> Self {
        Self {
            positionals,
            ..Self::default()
        }
    }

    /// Reads the next token out of the given parser.
    ///
    /// If the command's signature sets a maximum number of positionals (via `raw_after`) then
    /// the token may contain the rest of the parser's input.
    pub fn read_token<'p>(
        &mut self,
        parser: &mut Tokenizer<'p>,
    ) -> Result<Option<Token<'p>>, ParseArgsError<'p>> {
        if self
            .signature
            .raw_after
            .is_some_and(|max| self.len() >= max as usize)
        {
            self.only_positionals = true;
            Ok(parser.rest())
        } else {
            parser.next().transpose()
        }
    }

    /// Parses the given command line according to a command's signature.
    ///
    /// The `try_map_fn` function can be used to try changing each token before it is considered
    /// as an argument - this is used for variable expansion.
    pub fn parse<M>(
        line: &'a str,
        signature: Signature,
        validate: bool,
        mut try_map_fn: M,
    ) -> Result<Self, Box<dyn Error + 'a>>
    where
        // Note: this is a `FnMut` in case we decide to allow caching expansions in the future.
        // The `mut` is not currently used.
        M: FnMut(Token<'a>) -> Result<Cow<'a, str>, Box<dyn Error>>,
    {
        let mut tokenizer = Tokenizer::new(line, validate);
        let mut args = Self::new(signature, validate);

        while let Some(token) = args.read_token(&mut tokenizer)? {
            let arg = try_map_fn(token)?;
            args.push(arg)?;
        }

        args.finish()?;

        Ok(args)
    }

    /// Adds the given argument token.
    ///
    /// Once all arguments have been added, `Self::finish` should be called to perform any
    /// closing validations.
    pub fn push(&mut self, arg: Cow<'a, str>) -> Result<(), ParseArgsError<'a>> {
        if !self.only_positionals && arg == "--" {
            // "--" marks the end of flags, everything after is a positional even if it starts
            // with '-'.
            self.only_positionals = true;
            self.state = CompletionState::Flag(None);
        } else if let Some(flag) = self.flag_awaiting_argument() {
            // If the last token was a flag which accepts an argument, treat this token as a flag
            // argument.
            self.flags.insert(flag.name, arg);
            self.state = CompletionState::FlagArgument(flag);
        } else if !self.only_positionals && arg.starts_with('-') {
            // If the token starts with '-' and we are not only accepting positional arguments,
            // treat this token as a flag.
            let flag = if let Some(longhand) = arg.strip_prefix("--") {
                self.signature
                    .flags
                    .iter()
                    .find(|flag| flag.name == longhand)
            } else {
                let shorthand = arg.strip_prefix('-').unwrap();
                self.signature.flags.iter().find(|flag| {
                    flag.alias
                        .is_some_and(|ch| shorthand == ch.encode_utf8(&mut [0; 4]))
                })
            };

            let Some(flag) = flag else {
                if self.validate {
                    return Err(ParseArgsError::UnknownFlag { text: arg });
                }

                self.positionals.push(arg);
                self.state = CompletionState::Flag(None);
                return Ok(());
            };

            if self.validate && self.flags.contains_key(flag.name) {
                return Err(ParseArgsError::DuplicatedFlag { flag: flag.name });
            }

            self.flags.insert(flag.name, Cow::Borrowed(""));
            self.state = CompletionState::Flag(Some(*flag));
        } else {
            // Otherwise this token is a positional argument.
            self.positionals.push(arg);
            self.state = CompletionState::Positional;
        }

        Ok(())
    }

    /// Performs any validations that must be done after the input args are finished being pushed
    /// with `Self::push`.
    fn finish(&self) -> Result<(), ParseArgsError<'a>> {
        if !self.validate {
            return Ok(());
        };

        if let Some(flag) = self.flag_awaiting_argument() {
            return Err(ParseArgsError::FlagMissingArgument { flag: flag.name });
        }
        self.signature
            .check_positional_count(self.positionals.len())?;

        Ok(())
    }

    fn flag_awaiting_argument(&self) -> Option<Flag> {
        match self.state {
            CompletionState::Flag(flag) => flag.filter(|f| f.completions.is_some()),
            _ => None,
        }
    }

    /// Returns the kind of argument the last token is considered to be.
    ///
    /// For example if the last argument in the command line is `--foo` then the argument may be
    /// considered to be a flag.
    pub fn completion_state(&self) -> CompletionState {
        self.state
    }

    /// Returns the number of positionals supplied in the input.
    ///
    /// This number does not account for any flags passed in the input.
    pub fn len(&self) -> usize {
        self.positionals.len()
    }

    /// Checks whether the arguments contain no positionals.
    ///
    /// Note that this function returns `true` if there are no positional arguments even if the
    /// input contained flags.
    pub fn is_empty(&self) -> bool {
        self.positionals.is_empty()
    }

    /// Gets the first positional argument, if one exists.
    pub fn first(&'a self) -> Option<&'a str> {
        self.positionals.first().map(AsRef::as_ref)
    }

    /// Gets the positional argument at the given index, if one exists.
    pub fn get(&'a self, index: usize) -> Option<&'a str> {
        self.positionals.get(index).map(AsRef::as_ref)
    }

    /// Flattens all positional arguments together with the given separator between each
    /// positional.
    pub fn join(&self, sep: &str) -> String {
        self.positionals.join(sep)
    }

    /// Returns an iterator over all positional arguments.
    pub fn iter(&self) -> slice::Iter<'_, Cow<'_, str>> {
        self.positionals.iter()
    }

    /// Gets the value associated with a flag's long name if the flag was provided.
    ///
    /// This function should be preferred over [Self::has_flag] when the flag accepts an argument.
    pub fn get_flag(&'a self, name: &'static str) -> Option<&'a str> {
        debug_assert!(
            self.signature.flags.iter().any(|flag| flag.name == name),
            "flag '--{name}' does not belong to the command's signature"
        );
        debug_assert!(
            self.signature
                .flags
                .iter()
                .any(|flag| flag.name == name && flag.completions.is_some()),
            "Args::get_flag was used for '--{name}' but should only be used for flags with arguments, use Args::has_flag instead"
        );

        self.flags.get(name).map(AsRef::as_ref)
    }

    /// Checks if a flag was provided in the arguments.
    ///
    /// This function should be preferred over [Self::get_flag] for boolean flags - flags that
    /// either are present or not.
    pub fn has_flag(&self, name: &'static str) -> bool {
        debug_assert!(
            self.signature.flags.iter().any(|flag| flag.name == name),
            "flag '--{name}' does not belong to the command's signature"
        );
        debug_assert!(
            self.signature
                .flags
                .iter()
                .any(|flag| flag.name == name && flag.completions.is_none()),
            "Args::has_flag was used for '--{name}' but should only be used for flags without arguments, use Args::get_flag instead"
        );

        self.flags.contains_key(name)
    }
}

// `arg[n]`
impl ops::Index<usize> for Args<'_> {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.positionals[index].as_ref()
    }
}

// `for arg in args { .. }`
impl<'a> IntoIterator for Args<'a> {
    type Item = Cow<'a, str>;
    type IntoIter = vec::IntoIter<Cow<'a, str>>;

    fn into_iter(self) -> Self::IntoIter {
        self.positionals.into_iter()
    }
}

// `for arg in &args { .. }`
impl<'i, 'a> IntoIterator for &'i Args<'a> {
    type Item = &'i Cow<'a, str>;
    type IntoIter = slice::Iter<'i, Cow<'a, str>>;

    fn into_iter(self) -> Self::IntoIter {
        self.positionals.iter()
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

    pub fn parse_signature<'a>(
        input: &'a str,
        signature: Signature,
    ) -> Result<Args<'a>, Box<dyn std::error::Error + 'a>> {
        Args::parse(input, signature, true, |token| Ok(token.content))
    }

    #[test]
    fn signature_validation_positionals() {
        let signature = Signature {
            positionals: (2, Some(3)),
            ..Signature::DEFAULT
        };

        assert!(parse_signature("hello world", signature).is_ok());
        assert!(parse_signature("foo bar baz", signature).is_ok());
        assert!(parse_signature(r#"a "b c" d"#, signature).is_ok());

        assert!(parse_signature("hello", signature).is_err());
        assert!(parse_signature("foo bar baz quiz", signature).is_err());

        let signature = Signature {
            positionals: (1, None),
            ..Signature::DEFAULT
        };

        assert!(parse_signature("a", signature).is_ok());
        assert!(parse_signature("a b", signature).is_ok());
        assert!(parse_signature(r#"a "b c" d"#, signature).is_ok());

        assert!(parse_signature("", signature).is_err());
    }

    #[test]
    fn flags() {
        let signature = Signature {
            positionals: (1, Some(2)),
            flags: &[
                Flag {
                    name: "foo",
                    alias: Some('f'),
                    doc: "",
                    completions: None,
                },
                Flag {
                    name: "bar",
                    alias: Some('b'),
                    doc: "",
                    completions: Some(&[]),
                },
            ],
            ..Signature::DEFAULT
        };

        let args = parse_signature("hello", signature).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(&args[0], "hello");
        assert!(!args.has_flag("foo"));
        assert!(args.get_flag("bar").is_none());

        let args = parse_signature("--bar abcd hello world --foo", signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "hello");
        assert_eq!(&args[1], "world");
        assert!(args.has_flag("foo"));
        assert_eq!(args.get_flag("bar"), Some("abcd"));

        let args = parse_signature("hello -f -b abcd world", signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "hello");
        assert_eq!(&args[1], "world");
        assert!(args.has_flag("foo"));
        assert_eq!(args.get_flag("bar"), Some("abcd"));

        // The signature requires at least one positional.
        assert!(parse_signature("--foo", signature).is_err());
        // And at most two.
        assert!(parse_signature("abc --bar baz def efg", signature).is_err());

        let args = parse_signature(r#"abc -b "xyz 123" def"#, signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "abc");
        assert_eq!(&args[1], "def");
        assert_eq!(args.get_flag("bar"), Some("xyz 123"));

        // Unknown flags are validation errors.
        assert!(parse_signature(r#"foo --quiz"#, signature).is_err());
        // Duplicated flags are parsing errors.
        assert!(parse_signature(r#"--foo bar --foo"#, signature).is_err());
        assert!(parse_signature(r#"-f bar --foo"#, signature).is_err());

        // "--" can be used to mark the end of flags. Everything after is considered a positional.
        let args = parse_signature(r#"hello --bar baz -- --foo"#, signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "hello");
        assert_eq!(&args[1], "--foo");
        assert_eq!(args.get_flag("bar"), Some("baz"));
        assert!(!args.has_flag("foo"));
    }

    #[test]
    fn raw_after() {
        let signature = Signature {
            positionals: (1, Some(1)),
            raw_after: Some(0),
            ..Signature::DEFAULT
        };

        // All quoting and escaping is treated literally in raw mode.
        let args = parse_signature(r#"'\'"#, signature).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(&args[0], "'\\'");
        let args = parse_signature(r#"\''"#, signature).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(&args[0], "\\''");

        // Leading space is trimmed.
        let args = parse_signature(r#"   %sh{foo}"#, signature).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(&args[0], "%sh{foo}");

        let signature = Signature {
            positionals: (1, Some(2)),
            raw_after: Some(1),
            ..Signature::DEFAULT
        };

        let args = parse_signature("foo", signature).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(&args[0], "foo");

        // "--bar" is treated as a positional.
        let args = parse_signature("foo --bar", signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "foo");
        assert_eq!(&args[1], "--bar");

        let args = parse_signature("abc def ghi", signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "abc");
        assert_eq!(&args[1], "def ghi");

        let args = parse_signature("rulers [20, 30]", signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "rulers");
        assert_eq!(&args[1], "[20, 30]");

        let args =
            parse_signature(r#"gutters ["diff"] ["diff", "diagnostics"]"#, signature).unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(&args[0], "gutters");
        assert_eq!(&args[1], r#"["diff"] ["diff", "diagnostics"]"#);
    }
}
