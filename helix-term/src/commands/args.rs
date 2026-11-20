//! Argument parsing for `TypedCommands`.
//!
//! It interprets a stream of `Token`s according to a command's declared `Signature`,
//! separating positional arguments from flags and
//! performing all relevant validations.
//!
//! # Overview
//!
//! * `Flag` and `Signature` declare how a command's input should be structured.
//! * `Args` is the interface used by typable command implementations. It may be treated as a
//!   slice of `Cow<str>` or `&str` to access positional arguments — `for arg in args` iterates
//!   over positionals (never flags) and `&args[0]` always corresponds to the first positional.
//!   Use `Args::has_flag` and `Args::get_flag` to read any specified flags.
//! * `ParseArgsError` covers argument-level validation failures (wrong positional count,
//!   unknown or duplicated flags, etc.). Tokenization-level errors (`UnterminatedToken`,
//!   `MissingExpansionDelimiter`, `UnknownExpansion`) remain in
//!   `helix_core::command_line::TokenizeError`.
//!
//! Because this crate has access to command-specific metadata, `Flag` can freely reference
//! `CommandCompleter` and similar types defined in `helix-term`.

use std::{borrow::Cow, collections::HashMap, error::Error, fmt, ops, slice, vec};

use helix_core::command_line::{Token, TokenizeError, Tokenizer};

use crate::commands::CommandCompleter;

/// A Unix-like flag that a command may accept.
///
/// For example the `:sort` command accepts a `--reverse` (or `-r` for shorthand) boolean flag
/// which controls the direction of sorting. Flags may accept an argument by setting the
/// `completer` field to `Some`.
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
    /// The completer to use when specifying an argument for a flag.
    ///
    /// This should be set to `None` for boolean flags
    /// For args with completer, refer to `CommandCompleter` docs.
    pub completer: Option<CommandCompleter>,
}

impl Flag {
    // This allows defining flags with the `..Flag::DEFAULT` shorthand. The `name` and `doc`
    // fields should always be overwritten.
    pub const DEFAULT: Self = Self {
        name: "",
        doc: "",
        alias: None,
        completer: None,
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
    DuplicatedFlag {
        flag: &'static str,
    },
    UnknownFlag {
        text: Cow<'a, str>,
    },
    FlagMissingArgument {
        flag: &'static str,
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
            Self::DuplicatedFlag { flag } => {
                write!(f, "flag '--{flag}' specified more than once")
            }
            Self::UnknownFlag { text } => write!(f, "unknown flag '{text}'"),
            Self::FlagMissingArgument { flag } => {
                write!(f, "flag '--{flag}' missing an argument")
            }
        }
    }
}

impl Error for ParseArgsError<'_> {}

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

    /// Reads the next token out of the given parser.
    ///
    /// If the command's signature sets a maximum number of positionals (via `raw_after`) then
    /// the token may contain the rest of the parser's input.
    pub fn read_token<'p>(
        &mut self,
        parser: &mut Tokenizer<'p>,
    ) -> Result<Option<Token<'p>>, TokenizeError<'p>> {
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
            CompletionState::Flag(flag) => flag.filter(|f| f.completer.is_some()),
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
                .any(|flag| flag.name == name && flag.completer.is_some()),
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
                .any(|flag| flag.name == name && flag.completer.is_none()),
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

    fn parse_signature<'a>(
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
        static FLAGS: &[Flag] = &[
            Flag {
                name: "foo",
                alias: Some('f'),
                doc: "",
                completer: None,
            },
            Flag {
                name: "bar",
                alias: Some('b'),
                doc: "",
                completer: Some(CommandCompleter::none()),
            },
        ];

        let signature = Signature {
            positionals: (1, Some(2)),
            flags: FLAGS,
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
