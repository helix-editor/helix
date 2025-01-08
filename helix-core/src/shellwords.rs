use smartstring::{LazyCompact, SmartString};
use std::{
    borrow::Cow,
    ops::{Index, RangeFrom},
};

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
/// # use helix_core::shellwords::{Shellwords, Args};
///
/// let shellwords = Shellwords::from(":o a b c");
/// let args = Args::from(shellwords.args());
///
/// assert_eq!("a", &args[0]);
/// assert_eq!("b", &args[1]);
/// assert_eq!("c", &args[2]);
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
        ArgsParser::from(self.args()).last().map_or(
            self.input.ends_with(' ') || self.input.ends_with('\t'),
            |last| {
                if cfg!(windows) {
                    self.input.ends_with(' ') || self.input.ends_with('\t')
                } else {
                    !(last.ends_with("\\ ") || last.ends_with("\\\t"))
                        && (self.input.ends_with(' ') || self.input.ends_with('\t'))
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

/// Represents different ways that arguments can be handled when parsing.
#[derive(Debug, Clone, Copy)]
pub enum ParseMode {
    /// Treat the entire input as one positional with minimal processing.
    /// (I.e. expand `\t` and `\n` but don't split on spaces or handle quotes.)
    Literal,
    /// Regular shellwords behavior: split the input into multiple parameters.
    Parameters,
}

/// An abstraction for arguments that were passed in to a command.
#[derive(Debug)]
pub struct Args<'a> {
    input: &'a str,
    positionals: Vec<Cow<'a, str>>,
}

impl<'a> Args<'a> {
    /// Creates an instance of `Args`, with behavior shaped from a signature.
    #[inline]
    pub fn from_signature(args: &'a str, mode: ParseMode) -> anyhow::Result<Self> {
        // TODO: Extract flags into `HashMap`
        // let mut flags: HashMap<Cow<'_, str>, Cow<'_, str>> = HashMap::new();
        // Remove `with_unescaping` here and then do it down below.
        let positionals = ArgsParser::from(args);

        let args = match mode {
            ParseMode::Literal => Args {
                input: args,
                positionals: vec![unescape(args, false)],
                // TODO: flags: flags,
            },
            ParseMode::Parameters => Args {
                input: args,
                positionals: positionals
                    .with_unescaping()
                    .unescape_backslashes()
                    .collect(),
                // TODO: flags: flags,
            },
        };

        Ok(args)
    }

    /// Returns the count of how many arguments there are.
    #[inline]
    pub fn len(&self) -> usize {
        self.positionals.len()
    }

    /// Returns if there were no arguments passed in.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positionals.is_empty()
    }

    /// Returns a reference to an element if one exists at the index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Cow<'_, str>> {
        self.positionals.get(index)
    }

    /// Returns the first argument, if any.
    #[inline]
    pub fn first(&self) -> Option<&Cow<'_, str>> {
        self.positionals.first()
    }

    /// Returns the last argument, if any.
    #[inline]
    pub fn last(&self) -> Option<&Cow<'_, str>> {
        self.positionals.last()
    }

    /// Returns an instance of an `ArgsParser` iterator.
    ///
    /// For cases that need special handling, this could function as an
    /// escape hatch to further control the process of parsing out arguments and flags.
    #[inline]
    pub fn raw_parser(&self) -> ArgsParser<'_, false, false> {
        ArgsParser::from(self.input)
    }

    /// Produces an `Iterator` over the arguments that were passed along.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Cow<'_, str>> {
        self.positionals.iter()
    }

    /// Represents when there are no arguments.
    #[inline(always)]
    pub fn empty() -> Self {
        Self {
            input: "",
            positionals: Vec::new(),
        }
    }
}

// NOTE: When created with `from`, none but the most basic unescaping happens.
impl<'a> From<&'a String> for Args<'a> {
    #[inline]
    fn from(args: &'a String) -> Self {
        Args {
            input: args,
            positionals: ArgsParser::from(args).collect(),
        }
    }
}

impl<'a> From<&'a str> for Args<'a> {
    #[inline]
    fn from(args: &'a str) -> Self {
        Args {
            input: args,
            positionals: ArgsParser::from(args).collect(),
        }
    }
}

impl<'a> From<&'a Cow<'_, str>> for Args<'a> {
    #[inline]
    fn from(args: &'a Cow<str>) -> Self {
        Args {
            input: args,
            positionals: ArgsParser::from(args).collect(),
        }
    }
}

impl<'a> IntoIterator for Args<'a> {
    type Item = Cow<'a, str>;
    type IntoIter = std::vec::IntoIter<Cow<'a, str>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.positionals.into_iter()
    }
}

impl<'a> IntoIterator for &'a Args<'a> {
    type Item = &'a Cow<'a, str>;
    type IntoIter = std::slice::Iter<'a, Cow<'a, str>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.positionals.iter()
    }
}

impl<'a> AsRef<[Cow<'a, str>]> for Args<'a> {
    #[inline]
    fn as_ref(&self) -> &[Cow<'a, str>] {
        self.positionals.as_ref()
    }
}

impl PartialEq<&[&str]> for Args<'_> {
    #[inline]
    fn eq(&self, other: &&[&str]) -> bool {
        let this = self.positionals.iter();
        let other = other.iter().copied();

        for (left, right) in this.zip(other) {
            if left != right {
                return false;
            }
        }

        true
    }
}

impl<'a> Index<usize> for Args<'a> {
    type Output = str;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let cow = &self.positionals[index];
        cow.as_ref()
    }
}

impl<'a> Index<RangeFrom<usize>> for Args<'a> {
    type Output = [Cow<'a, str>];

    #[inline]
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.positionals[index]
    }
}

/// An iterator over an input string which yields arguments.
///
/// Splits on whitespace, but respects quoted substrings (using double quotes, single quotes, or backticks).
#[derive(Debug, Clone)]
pub struct ArgsParser<'a, const U: bool, const UB: bool> {
    input: &'a str,
    idx: usize,
    start: usize,
    unescape: bool,
    unescape_blackslash: bool,
}

impl<'a, const U: bool, const UB: bool> ArgsParser<'a, U, UB> {
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Returns the args exactly as input.
    ///
    /// # Examples
    /// ```
    /// # use helix_core::shellwords::ArgsParser;
    /// let args = ArgsParser::from(r#"sed -n "s/test t/not /p""#);
    /// assert_eq!(r#"sed -n "s/test t/not /p""#, args.raw());
    ///
    /// let args = ArgsParser::from(r#"cat "file name with space.txt""#);
    /// assert_eq!(r#"cat "file name with space.txt""#, args.raw());
    /// ```
    #[inline]
    pub const fn raw(&self) -> &str {
        self.input
    }

    /// Returns the remainder of the args exactly as input.
    ///
    /// # Examples
    /// ```
    /// # use helix_core::shellwords::ArgsParser;
    /// let mut args = ArgsParser::from(r#"sed -n "s/test t/not /p""#);
    /// assert_eq!("sed", args.next().unwrap());
    /// assert_eq!(r#"-n "s/test t/not /p""#, args.rest());
    /// ```
    ///
    /// Never calling `next` and using `rest` is functionally equivalent to calling `raw`.
    #[inline]
    pub fn rest(&self) -> &str {
        &self.input[self.idx..]
    }

    /// Returns a reference to the `next()` value without advancing the iterator.
    ///
    /// Unlike `std::iter::Peakable::peek` this does not return a double reference, `&&str`
    /// but a normal `&str`.
    #[inline]
    #[must_use]
    pub fn peek(&self) -> Option<Cow<'_, str>> {
        self.clone().next()
    }
}

impl<'a> ArgsParser<'a, false, false> {
    #[inline]
    const fn parse(input: &'a str) -> ArgsParser<'a, false, false> {
        ArgsParser::<'a, false, false> {
            input,
            idx: 0,
            start: 0,
            unescape: false,
            unescape_blackslash: false,
        }
    }

    #[inline]
    #[must_use]
    pub const fn with_unescaping(self) -> ArgsParser<'a, true, false> {
        ArgsParser::<'a, true, false> {
            input: self.input,
            idx: self.idx,
            start: self.start,
            unescape: true,
            unescape_blackslash: self.unescape_blackslash,
        }
    }
}

impl<'a> ArgsParser<'a, true, false> {
    #[inline]
    #[must_use]
    pub const fn unescape_backslashes(self) -> ArgsParser<'a, true, true> {
        ArgsParser::<'a, true, true> {
            input: self.input,
            idx: self.idx,
            start: self.start,
            unescape: self.unescape,
            unescape_blackslash: true,
        }
    }
}

impl<'a, const U: bool, const UB: bool> Iterator for ArgsParser<'a, U, UB> {
    type Item = Cow<'a, str>;

    #[inline]
    #[allow(clippy::too_many_lines)]
    fn next(&mut self) -> Option<Self::Item> {
        // The parser loop is split into three main blocks to handle different types of input processing:
        //
        // 1. Quote block:
        //    - Detects an unescaped quote character, either starting an in-quote scan or, if already in-quote,
        //      locating the closing quote to return the quoted argument.
        //    - Handles cases where mismatched quotes are ignored and when quotes appear as the last character.
        //
        // 2. Whitespace block:
        //    - Handles arguments separated by whitespace (space or tab), respecting quotes so quoted phrases
        //      remain grouped together.
        //    - Splits arguments by whitespace when outside of a quoted context and updates boundaries accordingly.
        //
        // 3. Catch-all block:
        //    - Handles any other character, updating the `is_escaped` status if a backslash is encountered,
        //      advancing the loop to the next character.

        let bytes = self.input.as_bytes();
        let mut in_quotes = false;
        let mut quote = b'\0';
        let mut is_escaped = false;

        while self.idx < bytes.len() {
            match bytes[self.idx] {
                b'"' | b'\'' | b'`' if !is_escaped => {
                    if in_quotes {
                        // Found the proper closing quote, so can return the arg and advance the state along.
                        if bytes[self.idx] == quote {
                            let arg = &self.input[self.start..self.idx];
                            self.idx += 1;
                            self.start = self.idx;

                            let output = if self.unescape {
                                unescape(arg, true)
                            } else {
                                Cow::from(arg)
                            };

                            return Some(output);
                        }
                        // If quote does not match the type of the opening quote, then do nothing and advance.
                        self.idx += 1;
                    } else if self.idx == bytes.len() - 1 {
                        // Special case for when a quote is the last input in args.
                        // e.g: :read "file with space.txt""
                        // This preserves the quote as an arg:
                        // - `file with space`
                        // - `"`
                        let arg = &self.input[self.idx..];
                        self.idx = bytes.len();
                        self.start = bytes.len();

                        let output = if self.unescape {
                            unescape(arg, true)
                        } else {
                            Cow::from(arg)
                        };

                        return Some(output);
                    } else {
                        // Found opening quote.
                        in_quotes = true;
                        // Kind of quote that was found.
                        quote = bytes[self.idx];

                        if self.start < self.idx {
                            // When part of the input ends in a quote, `one two" three`, this properly returns the `two`
                            // before advancing to the quoted arg for the next iteration:
                            // - `one` <- previous arg
                            // - `two` <- this step
                            // - ` three` <- next arg
                            let arg = &self.input[self.start..self.idx];
                            self.idx += 1;
                            self.start = self.idx;

                            let output = if self.unescape {
                                unescape(arg, true)
                            } else {
                                Cow::from(arg)
                            };

                            return Some(output);
                        }

                        // Advance after quote.
                        self.idx += 1;
                        // Exclude quote from arg output.
                        self.start = self.idx;
                    }
                }
                b' ' | b'\t' if !in_quotes && !is_escaped => {
                    // Found a true whitespace separator that wasn't inside quotes.

                    // Check if there is anything to return or if its just advancing over whitespace.
                    // `start` will only be less than `idx` when there is something to return.
                    if self.start < self.idx {
                        let arg = &self.input[self.start..self.idx];
                        self.idx += 1;
                        self.start = self.idx;

                        let output = if self.unescape {
                            unescape(arg, true)
                        } else {
                            Cow::from(arg)
                        };

                        return Some(output);
                    }

                    // Advance beyond the whitespace.
                    self.idx += 1;

                    // This is where `start` will be set to the start of an arg boundary, either encountering a word
                    // boundary or a quote boundary. If it finds a quote, then it will be advanced again in that part
                    // of the code. Either way, all that remains for the check above will be to return a full arg.
                    self.start = self.idx;
                }
                _ => {
                    // If previous loop didn't find any backslash and was already escaped it will change to false
                    // as the backslash chain was broken.
                    //
                    // If the previous loop had no backslash escape, and found one this iteration, then its the start
                    // of an escape chain.
                    is_escaped = match (is_escaped, bytes[self.idx]) {
                        (false, b'\\') => true, // Set `is_escaped` if the current byte is a backslash
                        _ => false, //Reset `is_escaped` if it was true, otherwise keep `is_escaped` as false
                    };

                    // Advance to next `char`.
                    self.idx += 1;
                }
            }
        }

        // Fallback that catches when the loop would have exited but failed to return the arg between start and the end.
        if self.start < bytes.len() {
            let arg = &self.input[self.start..];
            self.start = bytes.len();

            let output = if self.unescape {
                unescape(arg, true)
            } else {
                Cow::from(arg)
            };

            return Some(output);
        }

        // All args have been parsed.
        None
    }
}

impl<'a> From<&'a String> for ArgsParser<'a, false, false> {
    #[inline]
    fn from(args: &'a String) -> Self {
        ArgsParser::parse(args)
    }
}

impl<'a> From<&'a str> for ArgsParser<'a, false, false> {
    #[inline]
    fn from(args: &'a str) -> Self {
        ArgsParser::parse(args)
    }
}

impl<'a> From<&'a Cow<'_, str>> for ArgsParser<'a, false, false> {
    #[inline]
    fn from(args: &'a Cow<str>) -> Self {
        ArgsParser::parse(args)
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
/// - `\\"` is converted to `"` (double-quote)
/// - `\\'` is converted to `'` (single-quote)
/// - `\\ ` is converted to ` ` (space)
/// - `\\u{...}` is converted to the corresponding Unicode character
///
/// Other escape sequences, such as `\\` followed by any character not listed above, will remain unchanged.
///
/// If input is invalid, for example if there is invalid unicode, \u{999999999}, it will return the input as is.
#[inline]
#[must_use]
fn unescape(input: &str, unescape_blackslash: bool) -> Cow<'_, str> {
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
                    ' ' => unescaped.push(' '),
                    '\'' => unescaped.push('\''),
                    '"' => unescaped.push('"'),
                    '`' => unescaped.push('`'),
                    'u' => {
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

        let parser = Args::from(shellwords.args());

        assert_eq!(Cow::from("single_word"), parser[0]);
        assert_eq!(Cow::from("twó"), parser[1]);
        assert_eq!(Cow::from("wörds"), parser[2]);
        assert_eq!(Cow::from(r#"\\three\ \"with\ escaping\\"#), parser[3]);
    }

    #[test]
    fn base_with_parser_mode() {
        let shellwords =
            Shellwords::from(r#":o single_word twó wörds \\three\ \"with\ escaping\\"#);

        assert_eq!(":o", shellwords.command());
        assert_eq!(
            r#"single_word twó wörds \\three\ \"with\ escaping\\"#,
            shellwords.args()
        );

        let parser = Args::from_signature(shellwords.args(), ParseMode::Parameters).unwrap();

        assert_eq!(Cow::from("single_word"), parser[0]);
        assert_eq!(Cow::from("twó"), parser[1]);
        assert_eq!(Cow::from("wörds"), parser[2]);
        assert_eq!(Cow::from(r#"\three "with escaping\"#), parser[3]);
    }

    #[test]
    fn should_split_args_no_slash_unescaping() {
        let input = r#"single_word twó wörds \\three\ \"with\ escaping\\"#;

        let args: Vec<Cow<'_, str>> = ArgsParser::from(input).collect();

        assert_eq!(
            vec![
                "single_word",
                "twó",
                "wörds",
                r#"\\three\ \"with\ escaping\\"#
            ],
            args
        );
    }

    #[test]
    fn should_have_empty_args() {
        let shellwords = Shellwords::from(":quit");
        assert!(
            shellwords.args().is_empty(),
            "args: `{:#?}`",
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
    fn should_preserve_quote_if_last_argument() {
        let shellwords = Shellwords::from(r#":read "file with space.txt"""#);
        let mut args = ArgsParser::from(shellwords.args());

        assert_eq!("file with space.txt", args.next().unwrap());
        assert_eq!(r#"""#, args.last().unwrap());
    }

    #[test]
    fn should_respect_escaped_quote_in_what_looks_like_non_closed_arg() {
        let shellwords = Shellwords::from(r":rename 'should be one \'argument");
        let mut args = ArgsParser::from(shellwords.args()).with_unescaping();

        assert_eq!(r"should be one 'argument", args.next().unwrap());
        assert_eq!(None, args.next());
    }

    #[test]
    fn should_escape_whitespace() {
        assert_eq!(
            Some(Cow::from("a ")),
            ArgsParser::from(r"a\ ").with_unescaping().next(),
        );
        assert_eq!(
            Some(Cow::from("a\t")),
            ArgsParser::from(r"a\t").with_unescaping().next(),
        );
        assert_eq!(
            Some(Cow::from("a b.txt")),
            ArgsParser::from(r"a\ b.txt").with_unescaping().next(),
        );
    }

    #[test]
    fn should_parse_args_even_with_leading_whitespace() {
        // Three spaces
        assert_eq!(Cow::from("a"), Args::from("   a")[0]);
    }

    #[test]
    fn should_peek_next_arg_and_not_consume() {
        let mut args = ArgsParser::parse("a");

        assert_eq!(Some(Cow::Borrowed("a")), args.peek());
        assert_eq!(Some(Cow::Borrowed("a")), args.next());
        assert_eq!(None, args.next());
    }

    #[test]
    fn should_parse_single_quotes_while_respecting_escapes() {
        let parser = ArgsParser::from(
            r#"'single_word' 'twó wörds' '' ' ''\\three\' \"with\ escaping\\' 'quote incomplete"#,
        )
        .with_unescaping();
        let expected = [
            "single_word",
            "twó wörds",
            "",
            " ",
            r#"\three' "with escaping\"#,
            "quote incomplete",
        ];

        for (expected, actual) in expected.into_iter().zip(parser) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn should_parse_double_quotes_while_respecting_escapes() {
        let parser = ArgsParser::from(
            r#""single_word" "twó wörds" "" "  ""\\three\' \"with\ escaping\\" "dquote incomplete"#,
        )
        .with_unescaping();
        let expected = [
            "single_word",
            "twó wörds",
            "",
            "  ",
            r#"\three' "with escaping\"#,
            "dquote incomplete",
        ];

        for (expected, actual) in expected.into_iter().zip(parser) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn should_respect_escapes_with_mixed_quotes() {
        let parser = ArgsParser::from(r#"single_word 'twó wörds' "\\three\' \"with\ escaping\\""no space before"'and after' $#%^@ "%^&(%^" ')(*&^%''a\\\\\b' '"#).with_unescaping();
        let expected = [
            "single_word",
            "twó wörds",
            r#"\three' "with escaping\"#,
            "no space before",
            "and after",
            "$#%^@",
            "%^&(%^",
            r")(*&^%",
            r"a\\\b",
            // Last ' is important, as if the user input an accidental quote at the end, this should be checked in
            // commands where there should only be one input and return an error rather than silently succeed.
            "'",
        ];

        for (expected, actual) in expected.into_iter().zip(parser) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn should_return_rest_from_parser() {
        let mut parser = ArgsParser::from(r#"statusline.center ["file-type","file-encoding"]"#);

        assert_eq!(Some("statusline.center"), parser.next().as_deref());
        assert_eq!(r#"["file-type","file-encoding"]"#, parser.rest());
    }

    #[test]
    fn should_return_no_args() {
        let mut args = ArgsParser::parse("");
        assert!(args.next().is_none());
        assert!(args.is_empty());
    }

    #[test]
    fn should_leave_escaped_quotes() {
        let mut args = ArgsParser::parse(r#"\" \` \' \"with \'with \`with"#).with_unescaping();
        assert_eq!(Some(Cow::from(r#"""#)), args.next());
        assert_eq!(Some(Cow::from(r"`")), args.next());
        assert_eq!(Some(Cow::from(r"'")), args.next());
        assert_eq!(Some(Cow::from(r#""with"#)), args.next());
        assert_eq!(Some(Cow::from(r"'with")), args.next());
        assert_eq!(Some(Cow::from(r"`with")), args.next());
    }

    #[test]
    fn should_leave_literal_newline_alone() {
        let mut arg = ArgsParser::parse(r"\n").with_unescaping();
        assert_eq!(Some(Cow::from("\n")), arg.next());
    }

    #[test]
    fn should_leave_literal_unicode_alone() {
        let mut arg = ArgsParser::parse(r"\u{C}").with_unescaping();
        assert_eq!(Some(Cow::from("\u{C}")), arg.next());
    }

    #[test]
    fn should_escape_literal_unicode() {
        let mut arg = ArgsParser::parse(r"\u{C}");
        assert_eq!(Some(Cow::from("\\u{C}")), arg.next());
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
        let unescaped = unescape("hello\\nworld", true);
        assert_eq!("hello\nworld", unescaped);
    }

    #[test]
    fn should_unescape_tab() {
        let unescaped = unescape("hello\\tworld", true);
        assert_eq!("hello\tworld", unescaped);
    }

    #[test]
    fn should_unescape_unicode() {
        let unescaped = unescape("hello\\u{1f929}world", true);
        assert_eq!("hello\u{1f929}world", unescaped, "char: 🤩 ");
        assert_eq!("hello🤩world", unescaped);
    }

    #[test]
    fn should_return_original_input_due_to_bad_unicode() {
        let unescaped = unescape("hello\\u{999999999}world", true);
        assert_eq!("hello\\u{999999999}world", unescaped);
    }

    #[test]
    fn should_not_unescape_slash() {
        let unescaped = unescape(r"hello\\world", true);
        assert_eq!(r"hello\world", unescaped);

        let unescaped = unescape(r"hello\\\\world", true);
        assert_eq!(r"hello\\world", unescaped);
    }

    #[test]
    fn should_unescape_slash_single_quote() {
        let unescaped = unescape(r"\\'", true);
        assert_eq!(r"\'", unescaped);
    }

    #[test]
    fn should_unescape_slash_double_quote() {
        let unescaped = unescape(r#"\\\""#, true);
        assert_eq!(r#"\""#, unescaped);
    }

    #[test]
    fn should_not_change_anything() {
        let unescaped = unescape("'", true);
        assert_eq!("'", unescaped);
        let unescaped = unescape(r#"""#, true);
        assert_eq!(r#"""#, unescaped);
    }

    #[test]
    fn should_only_unescape_newline_not_slash_single_quote() {
        let unescaped = unescape("\\n\'", true);
        assert_eq!("\n'", unescaped);
        let unescaped = unescape(r"\\n\\'", true);
        assert_eq!(r"\n\'", unescaped);
    }

    #[test]
    fn should_unescape_args() {
        // 1f929: 🤩
        let args = ArgsParser::parse(r#"'hello\u{1f929} world' '["hello", "\u{1f929}", "world"]'"#)
            .with_unescaping()
            .collect::<Vec<_>>();

        assert_eq!("hello\u{1f929} world", unescape(&args[0], false));
        assert_eq!(r#"["hello", "🤩", "world"]"#, unescape(&args[1], false));
    }
}
