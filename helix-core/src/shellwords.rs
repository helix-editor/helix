use std::borrow::Cow;

use smartstring::{LazyCompact, SmartString};

/// A utility for parsing shell-like command lines.
///
/// The `Shellwords` struct takes an input string and allows extracting the command and its arguments.
///
/// # Features
///
/// - Parses command and arguments from input strings.
/// - Supports single, double, and backtick quoted arguments.
/// - Respects backslash escaping in arguments.
/// - Parses lists with `[]` syntax.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use helix_core::shellwords::Shellwords;
/// let shellwords = Shellwords::from(":o helix-core/src/shellwords.rs");
/// assert_eq!(":o", shellwords.command());
/// assert_eq!("helix-core/src/shellwords.rs", shellwords.args().next().unwrap());
/// ```
///
/// Empty command:
///
/// ```
/// # use helix_core::shellwords::Shellwords;
/// let shellwords = Shellwords::from(" ");
/// assert!(shellwords.command().is_empty());
/// ```
///
/// # Iterator
///
/// The `args` method returns a non-allocating iterator, `Args`, over the arguments of the input.
///
/// ```
/// # use helix_core::shellwords::Shellwords;
/// let shellwords = Shellwords::from(":o a b c");
/// let mut args = shellwords.args();
/// assert_eq!(Some("a"), args.next());
/// assert_eq!(Some("b"), args.next());
/// assert_eq!(Some("c"), args.next());
/// assert_eq!(None, args.next());
/// ```
#[derive(Clone, Copy)]
pub struct Shellwords<'a> {
    input: &'a str,
}

impl<'a> From<&'a str> for Shellwords<'a> {
    #[inline]
    fn from(input: &'a str) -> Self {
        Self { input }
    }
}

impl<'a> From<&'a String> for Shellwords<'a> {
    #[inline]
    fn from(input: &'a String) -> Self {
        Self { input }
    }
}

impl<'a> From<&'a Cow<'a, str>> for Shellwords<'a> {
    #[inline]
    fn from(input: &'a Cow<str>) -> Self {
        Self { input }
    }
}

impl<'a> Shellwords<'a> {
    #[inline]
    #[must_use]
    pub fn command(&self) -> &str {
        self.input
            .split_once(' ')
            .map_or(self.input, |(command, _)| command)
    }

    #[inline]
    #[must_use]
    pub fn args(&self) -> Args<'a> {
        let args = self.input.split_once(' ').map_or("", |(_, args)| args);
        Args::parse(args)
    }

    #[inline]
    pub fn input(&self) -> &str {
        self.input
    }

    /// Checks that the input ends with a whitespace character which is not escaped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use helix_core::shellwords::Shellwords;
    /// assert_eq!(Shellwords::from(" ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open foo.txt ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open").ends_with_whitespace(), false);
    /// assert_eq!(Shellwords::from(":open a\\ ").ends_with_whitespace(), true);
    /// assert_eq!(Shellwords::from(":open a\\ b.txt").ends_with_whitespace(), false);
    /// ```
    #[inline]
    pub fn ends_with_whitespace(&self) -> bool {
        self.input.ends_with(' ')
    }
}

/// An iterator over an input string which yields arguments.
///
/// Splits on whitespace, but respects quoted substrings (using double quotes, single quotes, or backticks).
/// It also handles lists enclosed in square brackets, treating them as a single argument.
#[derive(Debug, Clone, Copy)]
pub struct Args<'a> {
    input: &'a str,
    bytes: &'a [u8],
    idx: usize,
    start: usize,
    in_quotes: bool,
    quote: u8,
    in_list: bool,
    is_finished: bool,
}

impl<'a> Args<'a> {
    #[inline]
    fn parse(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            idx: 0,
            start: 0,
            in_quotes: false,
            quote: b'\0',
            in_list: false,
            is_finished: false,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Returns the args exactly as input.
    ///
    /// # Examples
    /// ```
    /// # use helix_core::shellwords::Args;
    /// let args = Args::from(r#"sed -n "s/test t/not /p""#);
    /// assert_eq!(r#"sed -n "s/test t/not /p""#, args.raw());
    ///
    /// let args = Args::from(r#"cat "file name with space.txt""#);
    /// assert_eq!(r#"cat "file name with space.txt""#, args.raw());
    /// ```
    #[inline]
    pub fn raw(&self) -> &str {
        self.input
    }

    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            input: "",
            bytes: &[],
            idx: 0,
            start: 0,
            in_quotes: false,
            quote: b'\0',
            in_list: false,
            is_finished: true,
        }
    }
}

#[allow(clippy::copy_iterator)]
impl<'a> Iterator for Args<'a> {
    type Item = &'a str;

    #[inline]
    #[allow(clippy::too_many_lines)]
    fn next(&mut self) -> Option<Self::Item> {
        /// Walks backwords checking for `\` and counting them to determine if escaped.
        #[inline]
        const fn is_escaped(bytes: &[u8]) -> bool {
            let mut backslashes = 0;
            let mut idx = bytes.len() - 1;

            loop {
                if bytes[idx] != b'\\' {
                    break;
                }
                backslashes += 1;

                idx = match idx.checked_sub(1) {
                    Some(idx) => idx,
                    None => break,
                };
            }

            (backslashes % 2) != 0
        }

        if self.is_finished {
            return None;
        }

        while self.idx < self.bytes.len() {
            match self.bytes[self.idx] {
                b'"' | b'\'' | b'`' if !self.in_list => {
                    if self.in_quotes {
                        if self.bytes[self.idx] == self.quote
                            && !is_escaped(&self.bytes[..self.idx])
                        {
                            let arg = Some(&self.input[self.start..self.idx]);
                            self.in_quotes = false;
                            self.quote = b'\0';
                            self.idx += 1;
                            self.start = self.idx;
                            return arg;
                        }
                        self.idx += 1;
                    } else if self.idx == self.bytes.len() - 1 {
                        // Special case for when a quote is the last input in args
                        // e.g: :yank-join , "
                        self.is_finished = true;
                        return Some(&self.input[self.idx..self.bytes.len()]);
                    } else if self.start < self.idx && !is_escaped(&self.bytes[..self.idx]) {
                        // When part of the input end in a quote, `one two" three` this returns the `two` properly.
                        let arg = Some(&self.input[self.start..self.idx]);
                        self.start = self.idx;
                        return arg;
                    } else if self.idx == 0 || !is_escaped(&self.bytes[..self.idx]) {
                        self.in_quotes = true;
                        self.quote = self.bytes[self.idx];
                        self.idx += 1;
                        // Exclude quote from arg output.
                        self.start = self.idx;

                        // Check if quote is ever closed, and if not, then return the rest of the input as one arg.
                        let mut found = false;
                        for idx in self.start..self.input.len() {
                            if self.bytes[idx] == self.quote && !is_escaped(&self.bytes[..self.idx])
                            {
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            self.is_finished = true;
                            return Some(&self.input[self.idx..]);
                        }
                    } else {
                        self.idx += 1;
                    }
                }
                b' ' | b'\t' if !self.in_quotes && !self.in_list => {
                    if self.idx + 1 == self.bytes.len() {
                        self.is_finished = true;
                        // Preserves whitespace if very last char of input.
                        return Some(&self.input[self.start..=self.idx]);
                    } else if self.start < self.idx {
                        let arg = Some(&self.input[self.start..self.idx]);
                        self.idx += 1;
                        self.start = self.idx;
                        return arg;
                    }
                    self.idx += 1;
                    self.start = self.idx;
                }
                b'[' if !self.in_quotes => {
                    self.in_list = true;
                    // Includes the `[` bracket
                    self.start = self.idx;
                    self.idx += 1;
                }
                b']' if self.in_list => {
                    self.in_list = false;
                    // Includes the `]` bracket
                    let arg = Some(&self.input[self.start..=self.idx]);
                    self.idx += 1;
                    self.start = self.idx;
                    return arg;
                }
                _ => {
                    self.idx += 1;
                }
            }
        }

        if self.start < self.bytes.len() {
            self.is_finished = true;
            return Some(&self.input[self.start..]);
        }

        None
    }
}

impl<'a> From<&'a String> for Args<'a> {
    fn from(args: &'a String) -> Self {
        Args::parse(args)
    }
}

impl<'a> From<&'a str> for Args<'a> {
    fn from(args: &'a str) -> Self {
        Args::parse(args)
    }
}

impl<'a> From<&'a Cow<'_, str>> for Args<'a> {
    fn from(args: &'a Cow<str>) -> Self {
        Args::parse(args)
    }
}

/// Auto escape for shellwords usage.
#[inline]
#[must_use]
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
        Cow::Owned(format!("\"{input}\""))
    }
}

/// Unescapes a string, converting escape sequences into their literal characters.
///
/// This function handles the following escape sequences:
/// - `\\n` is converted to `\n` (newline)
/// - `\\t` is converted to `\t` (tab)
/// - `\\u{...}` is converted to the corresponding Unicode character
///
/// Other escape sequences, such as `\\` followed by any character not listed above, will remain unchanged.
///
/// If input is invalid, for example if there is invalid unicode, \u{999999999}, it will return the input as is.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use helix_core::shellwords::unescape;
/// let unescaped = unescape("hello\\nworld");
/// assert_eq!("hello\nworld", unescaped);
/// ```
///
/// Unescaping tabs:
///
/// ```
/// # use helix_core::shellwords::unescape;
/// let unescaped = unescape("hello\\tworld");
/// assert_eq!("hello\tworld", unescaped);
/// ```
///
/// Unescaping Unicode characters:
///
/// ```
/// # use helix_core::shellwords::unescape;
/// let unescaped = unescape("hello\\u{1f929}world");
/// assert_eq!("hello\u{1f929}world", unescaped);
/// assert_eq!("hello🤩world", unescaped);
/// ```
///
/// Handling backslashes:
///
/// ```
/// # use helix_core::shellwords::unescape;
/// let unescaped = unescape(r"hello\\world");
/// assert_eq!(r"hello\\world", unescaped);
///
/// let unescaped = unescape(r"hello\\\\world");
/// assert_eq!(r"hello\\\\world", unescaped);
/// ```
///
/// # Note
///
/// This function is opinionated, with a clear purpose of handling user input, not a general or generic unescaping utility, and does not unescape sequences like `\\'` or `\\\"`, leaving them as is.
#[inline]
#[must_use]
pub fn unescape(input: &str) -> Cow<'_, str> {
    enum State {
        Normal,
        Escaped,
        Unicode,
    }

    let mut unescaped = String::new();
    let mut state = State::Normal;
    let mut is_escaped = false;
    // NOTE: Max unicode code point is U+10FFFF for a maximum of 6 chars
    let mut unicode = SmartString::<LazyCompact>::new_const();

    for (idx, ch) in input.char_indices() {
        match state {
            State::Normal => match ch {
                '\\' => {
                    if !is_escaped {
                        // PERF: As not every separator will be escaped, we use `String::new` as that has no initial
                        // allocation. If an escape is found, then we reserve capacity thats the len of the separator,
                        // as the new unescaped string will be at least that long.
                        unescaped.reserve(input.len());
                        if idx > 0 {
                            // First time finding an escape, so all prior chars can be added to the new unescaped
                            // version if its not the very first char found.
                            unescaped.push_str(&input[0..idx]);
                        }
                    }
                    state = State::Escaped;
                    is_escaped = true;
                }
                _ => {
                    if is_escaped {
                        unescaped.push(ch);
                    }
                }
            },
            State::Escaped => {
                match ch {
                    'n' => unescaped.push('\n'),
                    't' => unescaped.push('\t'),
                    'u' => {
                        state = State::Unicode;
                        continue;
                    }
                    // Uncomment if you want to handle '\\' to '\'
                    // '\\' => unescaped.push('\\'),
                    _ => {
                        unescaped.push('\\');
                        unescaped.push(ch);
                    }
                }
                state = State::Normal;
            }
            State::Unicode => match ch {
                '{' => continue,
                '}' => {
                    let Ok(digit) = u32::from_str_radix(&unicode, 16) else {
                        return input.into();
                    };
                    let Some(point) = char::from_u32(digit) else {
                        return input.into();
                    };
                    unescaped.push(point);
                    // Might be more unicode to unescape so clear for reuse.
                    unicode.clear();
                    state = State::Normal;
                }
                _ => unicode.push(ch),
            },
        }
    }

    if is_escaped {
        unescaped.into()
    } else {
        input.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn base() {
        let input = r#":o single_word twó wörds \three\ \"with\ escaping\\"#;
        let shellwords = Shellwords::from(input);
        let args = vec![
            "single_word",
            "twó",
            "wörds",
            r"\three\",
            r#"\"with\"#,
            r"escaping\\",
        ];

        assert_eq!(":o", shellwords.command());
        assert_eq!(args, shellwords.args().collect::<Vec<_>>());
    }

    #[test]
    fn should_have_empty_args() {
        let shellwords = Shellwords::from(":quit");
        assert!(
            shellwords.args().is_empty(),
            "args: `{}`",
            shellwords.args().next().unwrap()
        );
        assert!(shellwords.args().next().is_none());
    }

    #[test]
    fn should_return_empty_command() {
        let shellwords = Shellwords::from(" ");
        assert!(shellwords.command().is_empty());
    }

    #[test]
    fn should_support_unicode_args() {
        assert_eq!(
            Shellwords::from(":sh echo 𒀀").args().collect::<Vec<_>>(),
            &["echo", "𒀀"]
        );
        assert_eq!(
            Shellwords::from(":sh echo 𒀀 hello world𒀀")
                .args()
                .collect::<Vec<_>>(),
            &["echo", "𒀀", "hello", "world𒀀"]
        );
    }

    #[test]
    fn should_preserve_quote_if_only_argument() {
        let sh = Shellwords::from(r#":yank-join ""#);
        assert_eq!(r#"""#, sh.args().next().unwrap());
    }

    #[test]
    fn should_return_rest_of_non_closed_quote_as_one_argument() {
        let sh = Shellwords::from(r":rename 'should be one \'argument");
        assert_eq!(r"should be one \'argument", sh.args().next().unwrap());
    }

    #[test]
    fn should_split_args() {
        assert_eq!(Shellwords::from(":o a").args().collect::<Vec<_>>(), &["a"]);
        assert_eq!(
            Shellwords::from(":o a\\ ").args().collect::<Vec<_>>(),
            &["a\\ "]
        );
    }

    #[test]
    fn should_parse_single_quotes_while_respecting_escapes() {
        let quoted =
            r#":o 'single_word' 'twó wörds' '' ' ''\three\' \"with\ escaping\\' 'quote incomplete"#;
        let shellwords = Shellwords::from(quoted);
        let result = shellwords.args().collect::<Vec<_>>();
        let expected = vec![
            "single_word",
            "twó wörds",
            "",
            " ",
            r#"\three\' \"with\ escaping\\"#,
            "quote incomplete",
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn should_parse_double_quotes_while_respecting_escapes() {
        let dquoted = r#":o "single_word" "twó wörds" "" "  ""\three\' \"with\ escaping\\" "dquote incomplete"#;
        let shellwords = Shellwords::from(dquoted);
        let result = shellwords.args().collect::<Vec<_>>();
        let expected = vec![
            "single_word",
            "twó wörds",
            "",
            "  ",
            r#"\three\' \"with\ escaping\\"#,
            "dquote incomplete",
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn should_respect_escapes_with_mixed_quotes() {
        let dquoted = r#":o single_word 'twó wörds' "\three\' \"with\ escaping\\""no space before"'and after' $#%^@ "%^&(%^" ')(*&^%''a\\\\\b' '"#;
        let shellwords = Shellwords::from(dquoted);
        let result = shellwords.args().collect::<Vec<_>>();
        let expected = vec![
            "single_word",
            "twó wörds",
            r#"\three\' \"with\ escaping\\"#,
            "no space before",
            "and after",
            "$#%^@",
            "%^&(%^",
            r")(*&^%",
            r"a\\\\\b",
            // last ' is important, as if the user input an accidental quote at the end, this should be checked in
            // commands where there should only be one input and return an error rather than silently succeed.
            "'",
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn should_parse_lists() {
        let input =
            r#":set statusline.center ["file-type","file-encoding"] '["list", "in", "quotes"]'"#;
        let shellwords = Shellwords::from(input);
        let args = vec![
            "statusline.center",
            r#"["file-type","file-encoding"]"#,
            r#"["list", "in", "quotes"]"#,
        ];
        assert_eq!(":set", shellwords.command());
        assert_eq!(args, shellwords.args().collect::<Vec<_>>());
    }

    #[test]
    fn should_return_no_args() {
        let mut args = Args::parse("");
        assert!(args.next().is_none());
    }

    #[test]
    fn should_leave_escaped_quotes() {
        let input = r#"\" \` \' \"with \'with \`with"#;
        let result = Args::parse(input).collect::<Vec<_>>();
        assert_eq!(r#"\""#, result[0]);
        assert_eq!(r"\`", result[1]);
        assert_eq!(r"\'", result[2]);
        assert_eq!(r#"\"with"#, result[3]);
        assert_eq!(r"\'with", result[4]);
        assert_eq!(r"\`with", result[5]);
    }

    #[test]
    fn should_leave_literal_newline_alone() {
        let result = Args::parse(r"\n").collect::<Vec<_>>();
        assert_eq!(r"\n", result[0]);
    }

    #[test]
    fn should_leave_literal_unicode_alone() {
        let result = Args::parse(r"\u{C}").collect::<Vec<_>>();
        assert_eq!(r"\u{C}", result[0]);
    }

    #[test]
    fn should_parse_list() {
        let result = Args::parse(r#"["test", "list"]"#).collect::<Vec<_>>();
        assert_eq!(r#"["test", "list"]"#, result[0]);

        let result = Args::parse(r#"'["test", "list"]'"#).collect::<Vec<_>>();
        assert_eq!(r#"["test", "list"]"#, result[0]);
    }

    #[test]
    #[cfg(unix)]
    fn should_escape_unix() {
        assert_eq!(escape("foobar".into()), Cow::Borrowed("foobar"));
        assert_eq!(escape("foo bar".into()), Cow::Borrowed("foo\\ bar"));
        assert_eq!(escape("foo\tbar".into()), Cow::Borrowed("foo\\\tbar"));
    }

    #[test]
    #[cfg(windows)]
    fn should_escape_windows() {
        assert_eq!(escape("foobar".into()), Cow::Borrowed("foobar"));
        assert_eq!(escape("foo bar".into()), Cow::Borrowed("\"foo bar\""));
    }

    #[test]
    fn should_unescape_newline() {
        let unescaped = unescape("hello\\nworld");
        assert_eq!("hello\nworld", unescaped);
    }

    #[test]
    fn should_unescape_tab() {
        let unescaped = unescape("hello\\tworld");
        assert_eq!("hello\tworld", unescaped);
    }

    #[test]
    fn should_unescape_unicode() {
        let unescaped = unescape("hello\\u{1f929}world");
        assert_eq!("hello\u{1f929}world", unescaped, "char: 🤩 ");
        assert_eq!("hello🤩world", unescaped);
    }

    #[test]
    fn should_return_original_input_due_to_bad_unicode() {
        let unescaped = unescape("hello\\u{999999999}world");
        assert_eq!("hello\\u{999999999}world", unescaped);
    }

    #[test]
    fn should_not_unescape_slash() {
        let unescaped = unescape(r"hello\\world");
        assert_eq!(r"hello\\world", unescaped);

        let unescaped = unescape(r"hello\\\\world");
        assert_eq!(r"hello\\\\world", unescaped);
    }

    #[test]
    fn should_not_unescape_slash_single_quote() {
        let unescaped = unescape("\\'");
        assert_eq!(r"\'", unescaped);
    }

    #[test]
    fn should_not_unescape_slash_double_quote() {
        let unescaped = unescape("\\\"");
        assert_eq!(r#"\""#, unescaped);
    }

    #[test]
    fn should_not_change_anything() {
        let unescaped = unescape("'");
        assert_eq!("'", unescaped);
        let unescaped = unescape(r#"""#);
        assert_eq!(r#"""#, unescaped);
    }

    #[test]
    fn should_only_unescape_newline_not_slash_single_quote() {
        let unescaped = unescape("\\n\'");
        assert_eq!("\n'", unescaped);
        let unescaped = unescape("\\n\\'");
        assert_eq!("\n\\'", unescaped);
    }

    #[test]
    fn should_unescape_args() {
        // 1f929: 🤩
        let args = Args::parse(r#"'hello\u{1f929} world' ["hello", "\u{1f929}", "world"]"#)
            .collect::<Vec<_>>();
        assert_eq!("hello\u{1f929} world", unescape(args[0]));
        assert_eq!(r#"["hello", "🤩", "world"]"#, unescape(args[1]));
    }
}
