use std::{
    borrow::Cow,
    ops::{Index, RangeFrom},
};

use anyhow::ensure;

use crate::shellwords::unescape;

/// Represents different ways that arguments can be handled when parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    /// Treat the input as is, with no splitting or processing.
    Raw,
    /// Splits on whitespace while respected quoted substrings.
    ///
    /// Return value includes the start and end quotes.
    RawParams,
    /// Treat the entire input as one positional that only unescapes backslashes (I.e. resolve `\\` to `\` but not `\t` or `\n`).
    UnescapeBackslash,
    /// Split the input into multiple parameters, while escaping backslashes (`\\` to `\`), but not other literals.
    UnescapeBackslashParams,
    /// Treat the entire input as one positional with minimal processing (I.e. expand `\t` and `\n` but don't split on spaces or handle quotes).
    Literal,
    /// Treat the entire input as one positional with full processing (I.e. expand `\t`, `\n` and also `\\`).
    LiteralUnescapeBackslash,
    /// Split the input into multiple parameters, while escaping literals (`\n`, `\t`, `\u{C}`, etc.), but not backslashes.
    LiteralParams,
    /// Split the input into multiple parameters, while escaping literals (`\n`, `\t`, `\u{C}`, etc.), including backslashes.
    LiteralUnescapeBackslashParams,
}

#[derive(Clone)]
pub struct Signature {
    /// The min-max of the amount of positional arguments a command accepts.
    ///
    /// - **0**: (0, Some(0))
    /// - **0-1**: (0, Some(1))
    /// - **1**: (1, Some(1))
    /// - **1-10**: (1, Some(10))
    /// - **Unbounded**: (0, None)
    pub positionals: (usize, Option<usize>),
    pub parse_mode: ParseMode,
}

pub fn ensure_signature(name: &str, signature: &Signature, count: usize) -> anyhow::Result<()> {
    match signature.positionals {
        (0, Some(0)) => ensure!(count == 0, "`:{}` doesn't take any arguments", name),
        (min, Some(max)) if min == max => ensure!(
            (min..=max).contains(&count),
            "`:{}` needs `{min}` argument{}, got {count}",
            name,
            if min > 1 { "'s" } else { "" }
        ),
        (min, Some(max)) if min == max => ensure!(
            (min..=max).contains(&count),
            "`:{}` needs at least `{min}` argument{} and at most `{max}`, got `{count}`",
            name,
            if min > 1 { "'s" } else { "" }
        ),
        (min, _) => ensure!(
            (min..).contains(&count),
            "`:{}` needs at least `{min}` argument{}",
            name,
            if min > 1 { "'s" } else { "" }
        ),
    }

    Ok(())
}

/// An abstraction for arguments that were passed in to a command.
#[derive(Debug)]
pub struct Args<'a> {
    positionals: Vec<Cow<'a, str>>,
}

impl<'a> Args<'a> {
    /// Creates an instance of `Args`, with behavior shaped from a signature.
    #[inline]
    pub fn from_signature(
        name: &str,
        signature: &Signature,
        args: &'a str,
        validate: bool,
    ) -> anyhow::Result<Self> {
        let positionals: Vec<_> = ArgsParser::from(args)
            .with_mode(signature.parse_mode)
            .collect();

        if validate {
            ensure_signature(name, signature, positionals.len())?;
        }

        Ok(Args { positionals })
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

    /// Produces an `Iterator` over the arguments that were passed along.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Cow<'_, str>> {
        self.positionals.iter()
    }

    /// Represents when there are no arguments.
    #[inline(always)]
    pub fn empty() -> Self {
        Self {
            positionals: Vec::new(),
        }
    }
}

impl<'a> From<&'a str> for Args<'a> {
    #[inline]
    fn from(args: &'a str) -> Self {
        Args {
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
pub struct ArgsParser<'a> {
    input: &'a str,
    idx: usize,
    start: usize,
    mode: ParseMode,
    is_finished: bool,
}

impl<'a> ArgsParser<'a> {
    #[inline]
    const fn new(input: &'a str) -> ArgsParser<'a> {
        Self {
            input,
            idx: 0,
            start: 0,
            mode: ParseMode::Raw,
            is_finished: false,
        }
    }

    #[inline]
    #[must_use]
    pub fn with_mode(mut self, mode: ParseMode) -> ArgsParser<'a> {
        self.mode = mode;
        self
    }

    #[inline]
    pub fn set_mode(&mut self, mode: ParseMode) {
        self.mode = mode;
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Returns the args exactly as input.
    ///
    /// # Examples
    /// ```
    /// # use helix_core::args::ArgsParser;
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
    /// # use helix_core::args::{ArgsParser, ParseMode};
    /// let mut args = ArgsParser::from(r#"sed -n "s/test t/not /p""#).with_mode(ParseMode::RawParams);
    /// assert_eq!("sed", args.next().unwrap());
    /// assert_eq!(r#"-n "s/test t/not /p""#, args.rest());
    /// ```
    ///
    /// Never calling `next` and using `rest` is functionally equivalent to calling `raw`.
    #[inline]
    pub fn rest(&self) -> &str {
        &self.input[self.idx..]
    }
}

impl<'a> Iterator for ArgsParser<'a> {
    type Item = Cow<'a, str>;

    #[inline]
    #[allow(clippy::too_many_lines)]
    fn next(&mut self) -> Option<Self::Item> {
        // Special case so that `ArgsParser::new("")` and `Args::from("")` result in no iterations
        // being done, and `ArgsParser::new("").count == 0` and `Args::from("").is_empty` is `true`.
        if self.input.is_empty() {
            return None;
        }

        match self.mode {
            ParseMode::Raw if !self.is_finished => {
                self.start = self.input.len();
                self.idx = self.input.len();
                self.is_finished = true;

                return Some(Cow::from(self.input));
            }
            ParseMode::Literal if !self.is_finished => {
                self.start = self.input.len();
                self.idx = self.input.len();
                self.is_finished = true;

                return Some(unescape(self.input, true, false));
            }
            ParseMode::LiteralUnescapeBackslash if !self.is_finished => {
                self.start = self.input.len();
                self.idx = self.input.len();
                self.is_finished = true;

                return Some(unescape(self.input, true, true));
            }
            ParseMode::UnescapeBackslash if !self.is_finished => {
                self.start = self.input.len();
                self.idx = self.input.len();
                self.is_finished = true;

                return Some(unescape(self.input, false, true));
            }
            _ => {}
        }

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
                            let output = match self.mode {
                                ParseMode::RawParams => {
                                    // Include start and end quotes in return value
                                    let arg = &self.input[self.start - 1..=self.idx];
                                    self.idx += 1;
                                    self.start = self.idx;
                                    Cow::from(arg)
                                }
                                ParseMode::LiteralParams => {
                                    let arg = &self.input[self.start..self.idx];
                                    self.idx += 1;
                                    self.start = self.idx;

                                    unescape(arg, true, false)
                                }

                                ParseMode::LiteralUnescapeBackslashParams => {
                                    let arg = &self.input[self.start..self.idx];
                                    self.idx += 1;
                                    self.start = self.idx;

                                    unescape(arg, true, true)
                                }
                                ParseMode::UnescapeBackslashParams => {
                                    let arg = &self.input[self.start..self.idx];
                                    self.idx += 1;
                                    self.start = self.idx;

                                    unescape(arg, false, true)
                                }
                                _ => {
                                    unreachable!(
                                        "other variants are returned early at start of `next` {:?}",
                                        self.mode
                                    )
                                }
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

                        let output = match self.mode {
                            ParseMode::RawParams => Cow::from(arg),
                            ParseMode::LiteralParams => unescape(arg, true, false),
                            ParseMode::LiteralUnescapeBackslashParams => unescape(arg, true, true),
                            ParseMode::UnescapeBackslashParams => unescape(arg, false, true),
                            _ => {
                                unreachable!(
                                    "other variants are returned early at start of `next` {:?}",
                                    self.mode
                                )
                            }
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

                            let output = match self.mode {
                                ParseMode::RawParams => Cow::from(arg),
                                ParseMode::LiteralParams => unescape(arg, true, false),
                                ParseMode::LiteralUnescapeBackslashParams => {
                                    unescape(arg, true, true)
                                }

                                ParseMode::UnescapeBackslashParams => unescape(arg, false, true),
                                _ => {
                                    unreachable!(
                                        "other variants are returned early at start of `next` {:?}",
                                        self.mode
                                    )
                                }
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

                        let output = match self.mode {
                            ParseMode::RawParams => Cow::from(arg),
                            ParseMode::LiteralParams => unescape(arg, true, false),
                            ParseMode::LiteralUnescapeBackslashParams => unescape(arg, true, true),
                            ParseMode::UnescapeBackslashParams => unescape(arg, false, true),
                            _ => {
                                unreachable!(
                                    "other variants are returned early at start of `next` {:?}",
                                    self.mode
                                )
                            }
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

                    self.idx += 1;
                }
            }
        }

        // Fallback that catches when the loop would have exited but failed to return the arg between start and the end.
        if self.start < bytes.len() {
            let arg = &self.input[self.start..];

            let output = match self.mode {
                ParseMode::RawParams => {
                    Cow::from(&self.input[self.start - usize::from(in_quotes)..])
                }
                ParseMode::LiteralParams => unescape(arg, true, false),
                ParseMode::LiteralUnescapeBackslashParams => unescape(arg, true, true),
                ParseMode::UnescapeBackslashParams => unescape(arg, false, true),
                _ => {
                    unreachable!(
                        "other variants are returned early at start of `next` {:?}",
                        self.mode
                    )
                }
            };

            self.start = bytes.len();

            return Some(output);
        }

        // All args have been parsed.
        None
    }
}

impl<'a> From<&'a str> for ArgsParser<'a> {
    #[inline]
    fn from(args: &'a str) -> Self {
        ArgsParser::new(args)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_parse_arguments_with_no_unescaping() {
        let mut parser = ArgsParser::from(r#"single_word twó wörds \\three\ \"with\ escaping\\"#)
            .with_mode(ParseMode::RawParams);

        assert_eq!(Cow::from("single_word"), parser.next().unwrap());
        assert_eq!(Cow::from("twó"), parser.next().unwrap());
        assert_eq!(Cow::from("wörds"), parser.next().unwrap());
        assert_eq!(
            Cow::from(r#"\\three\ \"with\ escaping\\"#),
            parser.next().unwrap()
        );
    }

    #[test]
    fn should_honor_parser_mode() {
        let parser = Args::from_signature(
            "",
            &Signature {
                positionals: (0, None),
                parse_mode: ParseMode::LiteralUnescapeBackslashParams,
            },
            r#"single_word twó wörds \\three\ \"with\ escaping\\"#,
            true,
        )
        .unwrap();

        assert_eq!(Cow::from("single_word"), parser[0]);
        assert_eq!(Cow::from("twó"), parser[1]);
        assert_eq!(Cow::from("wörds"), parser[2]);
        assert_eq!(Cow::from(r#"\three "with escaping\"#), parser[3]);
    }

    #[test]
    fn should_split_args_no_slash_unescaping() {
        let args: Vec<Cow<'_, str>> =
            ArgsParser::from(r#"single_word twó wörds \\three\ \"with\ escaping\\"#)
                .with_mode(ParseMode::RawParams)
                .collect();

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
        let args = Args::from("");
        let mut parser = ArgsParser::new("");

        assert!(args.first().is_none());
        assert!(args.is_empty());
        assert!(parser.next().is_none());
        assert!(parser.is_empty());
    }

    #[test]
    fn should_preserve_quote_if_last_argument() {
        let mut args =
            ArgsParser::from(r#" "file with space.txt"""#).with_mode(ParseMode::LiteralParams);
        assert_eq!("file with space.txt", args.next().unwrap());
        assert_eq!(r#"""#, args.last().unwrap());
    }

    #[test]
    fn should_respect_escaped_quote_in_what_looks_like_non_closed_arg() {
        let mut args =
            ArgsParser::from(r"'should be one \'argument").with_mode(ParseMode::LiteralParams);
        assert_eq!(r"should be one 'argument", args.next().unwrap());
        assert_eq!(None, args.next());
    }

    #[test]
    fn should_escape_whitespace() {
        assert_eq!(
            Some(Cow::from("a ")),
            ArgsParser::from(r"a\ ")
                .with_mode(ParseMode::Literal)
                .next(),
        );
        assert_eq!(
            Some(Cow::from("a\t")),
            ArgsParser::from(r"a\t")
                .with_mode(ParseMode::Literal)
                .next(),
        );
        assert_eq!(
            Some(Cow::from("a b.txt")),
            ArgsParser::from(r"a\ b.txt")
                .with_mode(ParseMode::Literal)
                .next(),
        );
    }

    #[test]
    fn should_parse_args_even_with_leading_whitespace() {
        let mut parser = ArgsParser::new("   a").with_mode(ParseMode::RawParams);
        // Three spaces
        assert_eq!(Cow::from("a"), parser.next().unwrap());
    }

    #[test]
    fn should_parse_single_quotes_while_respecting_escapes() {
        let parser = ArgsParser::from(
            r#"'single_word' 'twó wörds' '' ' ''\\three\' \"with\ escaping\\' 'quote incomplete"#,
        )
        .with_mode(ParseMode::LiteralUnescapeBackslashParams);
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
        .with_mode(ParseMode::LiteralUnescapeBackslashParams);
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
        let args = ArgsParser::from(r#"single_word 'twó wörds' "\\three\' \"with\ escaping\\""no space before"'and after' $#%^@ "%^&(%^" ')(*&^%''a\\\\\b' '"#)
            .with_mode(ParseMode::LiteralUnescapeBackslashParams);
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

        for (expected, actual) in expected.into_iter().zip(args) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn should_return_rest_from_parser() {
        let mut parser = ArgsParser::from(r#"statusline.center ["file-type","file-encoding"]"#)
            .with_mode(ParseMode::RawParams);

        assert_eq!(Some("statusline.center"), parser.next().as_deref());
        assert_eq!(r#"["file-type","file-encoding"]"#, parser.rest());
    }

    #[test]
    fn should_leave_escaped_quotes() {
        let mut args =
            ArgsParser::new(r#"\" \` \' \"with \'with \`with"#).with_mode(ParseMode::LiteralParams);
        assert_eq!(Some(Cow::from(r#"""#)), args.next());
        assert_eq!(Some(Cow::from(r"`")), args.next());
        assert_eq!(Some(Cow::from(r"'")), args.next());
        assert_eq!(Some(Cow::from(r#""with"#)), args.next());
        assert_eq!(Some(Cow::from(r"'with")), args.next());
        assert_eq!(Some(Cow::from(r"`with")), args.next());
    }

    #[test]
    fn should_leave_literal_newline_alone() {
        let mut arg = ArgsParser::new(r"\n").with_mode(ParseMode::LiteralParams);
        assert_eq!(Some(Cow::from("\n")), arg.next());
    }

    #[test]
    fn should_leave_literal_unicode_alone() {
        let mut arg = ArgsParser::new(r"\u{C}").with_mode(ParseMode::LiteralParams);
        assert_eq!(Some(Cow::from("\u{C}")), arg.next());
    }

    #[test]
    fn should_escape_literal_unicode() {
        let mut arg = ArgsParser::new(r"\u{C}").with_mode(ParseMode::RawParams);
        assert_eq!(Some(Cow::from("\\u{C}")), arg.next());
    }

    #[test]
    fn should_unescape_args() {
        // 1f929: 🤩
        let args = ArgsParser::new(r#"'hello\u{1f929} world' '["hello", "\u{1f929}", "world"]'"#)
            .with_mode(ParseMode::LiteralParams)
            .collect::<Vec<_>>();

        assert_eq!("hello\u{1f929} world", unescape(&args[0], true, false));
        assert_eq!(
            r#"["hello", "🤩", "world"]"#,
            unescape(&args[1], true, false)
        );
    }

    #[test]
    fn should_parse_a_slash_b_correctly() {
        let args = ArgsParser::new(r"a\b")
            .with_mode(ParseMode::LiteralParams)
            .collect::<Vec<_>>();

        assert_eq!(r"a\b", &args[0]);
    }

    #[test]
    fn should_end_in_unterminated_quotes() {
        let mut args = ArgsParser::new(r#"a.txt "b "#).with_mode(ParseMode::RawParams);
        let last = args.by_ref().last();

        assert_eq!(Some(Cow::from(r#""b "#)), last);
    }

    #[test]
    fn should_end_with_raw_escaped_space() {
        let mut args = ArgsParser::new(r"helix-term\src\commands\typed.rs\ ");
        assert_eq!(
            Some(Cow::from(r"helix-term\src\commands\typed.rs\ ")),
            args.next()
        );
    }

    #[test]
    fn should_remain_in_bounds_when_raw_params_parsing_path() {
        let mut args = ArgsParser::from(r#""C:\\Users\\Helix\\AppData\\Local\\Temp\\.tmp3Dugy8""#);

        assert_eq!(
            Some(Cow::from(
                r#""C:\\Users\\Helix\\AppData\\Local\\Temp\\.tmp3Dugy8""#
            )),
            args.next()
        );
    }

    #[test]
    fn should_only_unescape_backslash() {
        let mut args = ArgsParser::from(r"C:\\Users\\Helix\\AppData\\Local\\Temp\\.tmp3Dugy8")
            .with_mode(ParseMode::UnescapeBackslash);

        assert_eq!(
            Some(Cow::from(r"C:\Users\Helix\AppData\Local\Temp\.tmp3Dugy8")),
            args.next()
        );
    }
    #[test]
    fn should_only_unescape_backslash_params() {
        let mut args = ArgsParser::from(r#""C:\\Users\\Helix\\AppData\\Local\\Temp\\.tmp3Dugy8""#)
            .with_mode(ParseMode::UnescapeBackslashParams);

        assert_eq!(
            Some(Cow::from(r"C:\Users\Helix\AppData\Local\Temp\.tmp3Dugy8")),
            args.next()
        );
    }
}
