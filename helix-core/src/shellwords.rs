use smartstring::{LazyCompact, SmartString};
use std::borrow::Cow;

use crate::args::{ArgsParser, ParseMode};

/// A utility for parsing shell-like command lines.
///
/// The `Shellwords` struct takes an input string and allows extracting the command and its arguments.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use helix_core::shellwords::Shellwords;
/// let shellwords = Shellwords::from(":o helix-core/src/shellwords.rs");
/// assert_eq!(":o", shellwords.command());
/// assert_eq!("helix-core/src/shellwords.rs", shellwords.args());
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
/// Arguments:
///
/// ```
/// # use helix_core::shellwords::Shellwords;
///
/// let shellwords = Shellwords::from(":o a b c");
/// assert_eq!("a b c", shellwords.args());
/// ```
#[derive(Clone, Copy)]
pub struct Shellwords<'a> {
    input: &'a str,
}

impl<'a> Shellwords<'a> {
    #[inline]
    #[must_use]
    pub fn command(&self) -> &str {
        self.input
            .split_once(' ')
            .map_or(self.input, |(command, _)| command)
    }

    /// Returns the ramining text after the command, splitting on horizontal whitespace.
    #[inline]
    #[must_use]
    pub fn args(&self) -> &str {
        self.input
            .split_once([' ', '\t'])
            .map_or("", |(_, args)| args)
    }

    /// Returns the input that was passed in to create a `Shellwords` instance exactly as is.
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
    /// assert_eq!(Shellwords::from(r#":open "a "#).ends_with_whitespace(), false);
    /// assert_eq!(Shellwords::from(":open a\\ b.txt").ends_with_whitespace(), false);
    /// #[cfg(windows)]
    /// assert_eq!(Shellwords::from(":open a\\\t").ends_with_whitespace(), true);
    /// #[cfg(windows)]
    /// assert_eq!(Shellwords::from(":open a\\ ").ends_with_whitespace(), true);
    /// #[cfg(unix)]
    /// assert_eq!(Shellwords::from(":open a\\ ").ends_with_whitespace(), false);
    /// #[cfg(unix)]
    /// assert_eq!(Shellwords::from(":open a\\\t").ends_with_whitespace(), false);
    /// ```
    #[inline]
    #[must_use]
    pub fn ends_with_whitespace(&self) -> bool {
        ArgsParser::from(self.args())
            .with_mode(ParseMode::RawParams)
            .last()
            .map_or(
                self.input.ends_with(' ') || self.input.ends_with('\t'),
                |last| {
                    if cfg!(windows) {
                        let ends_with_whitespace =
                            self.input.ends_with(' ') || self.input.ends_with('\t');
                        let last_starts_with_quote = last.starts_with('"')
                            || last.starts_with('\'')
                            || last.starts_with('`');

                        ends_with_whitespace && !last_starts_with_quote
                    } else {
                        let ends_with_escaped_whitespace =
                            last.ends_with("\\ ") || last.ends_with("\\\t");
                        let end_with_whitespace =
                            self.input.ends_with(' ') || self.input.ends_with('\t');
                        let last_starts_with_quote = last.starts_with('"')
                            || last.starts_with('\'')
                            || last.starts_with('`');
                        let ends_in_true_whitespace =
                            !ends_with_escaped_whitespace && end_with_whitespace;

                        ends_in_true_whitespace && !last_starts_with_quote
                    }
                },
            )
    }
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
        Cow::Owned(format!("\"{}\"", input))
    }
}

/// Unescapes a string, converting escape sequences into their literal characters.
///
/// This function handles the following escape sequences:
/// - `\\n` is converted to `\n` (newline)
/// - `\\t` is converted to `\t` (tab)
/// - `\\"` is converted to `"` (double-quote)
/// - `\\'` is converted to `'` (single-quote)
/// - `\\ ` is converted to ` ` (space)
/// - `\\u{...}` is converted to the corresponding Unicode character
/// - backticks are also converted the same as quotes.
///
/// Other escape sequences, such as `\\` followed by any character not listed above, will remain unchanged.
///
/// If input is invalid, for example if there is invalid unicode, \u{999999999}, it will return the input as is.
#[inline]
#[must_use]
pub(super) fn unescape(
    input: &str,
    unescape_literals: bool,
    unescape_blackslash: bool,
) -> Cow<'_, str> {
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
                    // Special case if last `char` encountered is a `\`
                    if idx + 1 == input.len() {
                        unescaped.push('\\');
                        break;
                    }

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
                    'n' if unescape_literals => unescaped.push('\n'),
                    't' if unescape_literals => unescaped.push('\t'),
                    ' ' if unescape_literals => unescaped.push(' '),
                    '\'' if unescape_literals => unescaped.push('\''),
                    '"' if unescape_literals => unescaped.push('"'),
                    '`' if unescape_literals => unescaped.push('`'),
                    'u' if unescape_literals => {
                        state = State::Unicode;
                        continue;
                    }
                    '\\' if unescape_blackslash => unescaped.push('\\'),
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
        let shellwords =
            Shellwords::from(r#":o single_word twó wörds \\three\ \"with\ escaping\\"#);

        assert_eq!(":o", shellwords.command());
        assert_eq!(
            r#"single_word twó wörds \\three\ \"with\ escaping\\"#,
            shellwords.args()
        );
    }

    #[test]
    fn should_return_empty_command() {
        let shellwords = Shellwords::from(" ");
        assert!(shellwords.command().is_empty());
    }

    #[test]
    fn should_support_unicode_args() {
        let shellwords = Shellwords::from(":yank-join 𒀀");
        assert_eq!(":yank-join", shellwords.command());
        assert_eq!(shellwords.args(), "𒀀");
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
        let unescaped = unescape("hello\\nworld", true, true);
        assert_eq!("hello\nworld", unescaped);
    }

    #[test]
    fn should_unescape_tab() {
        let unescaped = unescape("hello\\tworld", true, true);
        assert_eq!("hello\tworld", unescaped);
    }

    #[test]
    fn should_unescape_unicode() {
        let unescaped = unescape("hello\\u{1f929}world", true, true);
        assert_eq!("hello\u{1f929}world", unescaped, "char: 🤩 ");
        assert_eq!("hello🤩world", unescaped);
    }

    #[test]
    fn should_return_original_input_due_to_bad_unicode() {
        let unescaped = unescape("hello\\u{999999999}world", true, true);
        assert_eq!("hello\\u{999999999}world", unescaped);
    }

    #[test]
    fn should_not_unescape_slash() {
        let unescaped = unescape(r"hello\\world", true, true);
        assert_eq!(r"hello\world", unescaped);

        let unescaped = unescape(r"hello\\\\world", true, true);
        assert_eq!(r"hello\\world", unescaped);
    }

    #[test]
    fn should_unescape_slash_single_quote() {
        let unescaped = unescape(r"\\'", true, true);
        assert_eq!(r"\'", unescaped);
    }

    #[test]
    fn should_unescape_slash_double_quote() {
        let unescaped = unescape(r#"\\\""#, true, true);
        assert_eq!(r#"\""#, unescaped);
    }

    #[test]
    fn should_not_change_anything() {
        let unescaped = unescape("'", true, true);
        assert_eq!("'", unescaped);
        let unescaped = unescape(r#"""#, true, true);
        assert_eq!(r#"""#, unescaped);
    }

    #[test]
    fn should_only_unescape_newline_not_slash_single_quote() {
        let unescaped = unescape("\\n\'", true, true);
        assert_eq!("\n'", unescaped);
        let unescaped = unescape(r"\\n\\'", true, true);
        assert_eq!(r"\n\'", unescaped);
    }

    #[test]
    fn should_have_final_char_be_backslash() {
        assert_eq!(
            Cow::from(r"helix-term\"),
            unescape(r"helix-term\", true, false)
        );
        assert_eq!(
            Cow::from(r".git\info\"),
            unescape(r".git\info\", true, false)
        );
    }

    #[test]
    fn should_only_unescape_backslash() {
        assert_eq!(
            Cow::from(r"helix-term\"),
            unescape(r"helix-term\\", false, true)
        );
    }

    #[test]
    fn should_end_in_whitespace() {
        assert!(!Shellwords::from(r#":option "abc "#).ends_with_whitespace());
        assert!(!Shellwords::from(":option abc").ends_with_whitespace());
    }
}
