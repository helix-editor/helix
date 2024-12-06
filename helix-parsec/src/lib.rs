//! Parser-combinator functions
//!
//! This module provides parsers and parser combinators which can be used
//! together to build parsers by functional composition.

// This module implements parser combinators following https://bodil.lol/parser-combinators/.
// `sym` (trait implementation for `&'static str`), `map`, `pred` (filter), `one_or_more`,
// `zero_or_more`, as well as the `Parser` trait originate mostly from that post.
// The remaining parsers and parser combinators are either based on
// https://github.com/archseer/snippets.nvim/blob/a583da6ef130d2a4888510afd8c4e5ffd62d0dce/lua/snippet/parser.lua#L5-L138
// or are novel.

// When a parser matches the input successfully, it returns `Ok((next_input, some_value))`
// where the type of the returned value depends on the parser. If the parser fails to match,
// it returns `Err(input)`.
type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

/// A parser or parser-combinator.
///
/// Parser-combinators compose multiple parsers together to parse input.
/// For example, two basic parsers (`&'static str`s) may be combined with
/// a parser-combinator like [or] to produce a new parser.
///
/// ```
/// use helix_parsec::{or, Parser};
/// let foo = "foo"; // matches "foo" literally
/// let bar = "bar"; // matches "bar" literally
/// let foo_or_bar = or(foo, bar); // matches either "foo" or "bar"
/// assert_eq!(Ok(("", "foo")), foo_or_bar.parse("foo"));
/// assert_eq!(Ok(("", "bar")), foo_or_bar.parse("bar"));
/// assert_eq!(Err("baz"), foo_or_bar.parse("baz"));
/// ```
pub trait Parser<'a> {
    type Output;

    fn parse(&self, input: &'a str) -> ParseResult<'a, Self::Output>;
}

// Most parser-combinators are written as higher-order functions which take some
// parser(s) as input and return a new parser: a function that takes input and returns
// a parse result. The underlying implementation of [Parser::parse] for these functions
// is simply application.
#[doc(hidden)]
impl<'a, F, T> Parser<'a> for F
where
    F: Fn(&'a str) -> ParseResult<'a, T>,
{
    type Output = T;

    fn parse(&self, input: &'a str) -> ParseResult<'a, Self::Output> {
        self(input)
    }
}

/// A parser which matches the string literal exactly.
///
/// This parser succeeds if the next characters in the input are equal to the given
/// string literal.
///
/// Note that [str::parse] interferes with calling [Parser::parse] on string literals
/// directly; this trait implementation works when used within any parser combinator
/// but does not work on its own. To call [Parser::parse] on a parser for a string
/// literal, use the [token] parser.
///
/// # Examples
///
/// ```
/// use helix_parsec::{or, Parser};
/// let parser = or("foo", "bar");
/// assert_eq!(Ok(("", "foo")), parser.parse("foo"));
/// assert_eq!(Ok(("", "bar")), parser.parse("bar"));
/// assert_eq!(Err("baz"), parser.parse("baz"));
/// ```
impl<'a> Parser<'a> for &'static str {
    type Output = &'a str;

    fn parse(&self, input: &'a str) -> ParseResult<'a, Self::Output> {
        match input.get(0..self.len()) {
            Some(actual) if actual == *self => Ok((&input[self.len()..], &input[0..self.len()])),
            _ => Err(input),
        }
    }
}

// Parsers

/// A parser which matches the given string literally.
///
/// This function is a convenience for interpreting string literals as parsers
/// and is only necessary to avoid conflict with [str::parse]. See the documentation
/// for the `&'static str` implementation of [Parser].
///
/// # Examples
///
/// ```
/// use helix_parsec::{token, Parser};
/// let parser = token("foo");
/// assert_eq!(Ok(("", "foo")), parser.parse("foo"));
/// assert_eq!(Err("bar"), parser.parse("bar"));
/// ```
pub fn token<'a>(literal: &'static str) -> impl Parser<'a, Output = &'a str> {
    literal
}

/// A parser which matches all values until the specified pattern is found.
///
/// If the pattern is not found, this parser does not match. The input up to the
/// character which returns `true` is returned but not that character itself.
///
/// If the pattern function returns true on the first input character, this
/// parser fails.
///
/// # Examples
///
/// ```
/// use helix_parsec::{take_until, Parser};
/// let parser = take_until(|c| c == '.');
/// assert_eq!(Ok((".bar", "foo")), parser.parse("foo.bar"));
/// assert_eq!(Err(".foo"), parser.parse(".foo"));
/// assert_eq!(Err("foo"), parser.parse("foo"));
/// ```
pub fn take_until<'a, F>(pattern: F) -> impl Parser<'a, Output = &'a str>
where
    F: Fn(char) -> bool,
{
    move |input: &'a str| match input.find(&pattern) {
        Some(index) if index != 0 => Ok((&input[index..], &input[0..index])),
        _ => Err(input),
    }
}

/// A parser which matches all values until the specified pattern no longer match.
///
/// This parser only ever fails if the input has a length of zero.
///
/// # Examples
///
/// ```
/// use helix_parsec::{take_while, Parser};
/// let parser = take_while(|c| c == '1');
/// assert_eq!(Ok(("2", "11")), parser.parse("112"));
/// assert_eq!(Err("22"), parser.parse("22"));
/// ```
pub fn take_while<'a, F>(pattern: F) -> impl Parser<'a, Output = &'a str>
where
    F: Fn(char) -> bool,
{
    move |input: &'a str| match input
        .char_indices()
        .take_while(|(_p, c)| pattern(*c))
        .last()
    {
        Some((index, c)) => {
            let index = index + c.len_utf8();
            Ok((&input[index..], &input[0..index]))
        }
        _ => Err(input),
    }
}

// Variadic parser combinators

/// A parser combinator which matches a sequence of parsers in an all-or-nothing fashion.
///
/// The returned value is a tuple containing the outputs of all parsers in order. Each
/// parser in the sequence may be typed differently.
///
/// # Examples
///
/// ```
/// use helix_parsec::{seq, Parser};
/// let parser = seq!("<", "a", ">");
/// assert_eq!(Ok(("", ("<", "a", ">"))), parser.parse("<a>"));
/// assert_eq!(Err("<b>"), parser.parse("<b>"));
/// ```
#[macro_export]
macro_rules! seq {
    ($($parsers: expr),+ $(,)?) => {
        ($($parsers),+)
    }
}

// Seq is implemented using trait-implementations of Parser for various size tuples.
// This allows sequences to be typed heterogeneously.
macro_rules! seq_impl {
    ($($parser:ident),+) => {
        #[allow(non_snake_case)]
        impl<'a, $($parser),+> Parser<'a> for ($($parser),+)
        where
            $($parser: Parser<'a>),+
        {
            type Output = ($($parser::Output),+);

            fn parse(&self, input: &'a str) -> ParseResult<'a, Self::Output> {
                let ($($parser),+) = self;
                seq_body_impl!(input, input, $($parser),+ ; )
            }
        }
    }
}

macro_rules! seq_body_impl {
    ($input:expr, $next_input:expr, $head:ident, $($tail:ident),+ ; $(,)? $($acc:ident),*) => {
        match $head.parse($next_input) {
            Ok((next_input, $head)) => seq_body_impl!($input, next_input, $($tail),+ ; $($acc),*, $head),
            Err(_) => Err($input),
        }
    };
    ($input:expr, $next_input:expr, $last:ident ; $(,)? $($acc:ident),*) => {
        match $last.parse($next_input) {
            Ok((next_input, last)) => Ok((next_input, ($($acc),+, last))),
            Err(_) => Err($input),
        }
    }
}

seq_impl!(A, B);
seq_impl!(A, B, C);
seq_impl!(A, B, C, D);
seq_impl!(A, B, C, D, E);
seq_impl!(A, B, C, D, E, F);
seq_impl!(A, B, C, D, E, F, G);
seq_impl!(A, B, C, D, E, F, G, H);
seq_impl!(A, B, C, D, E, F, G, H, I);
seq_impl!(A, B, C, D, E, F, G, H, I, J);

/// A parser combinator which chooses the first of the input parsers which matches
/// successfully.
///
/// All input parsers must have the same output type. This is a variadic form for [or].
///
/// # Examples
///
/// ```
/// use helix_parsec::{choice, or, Parser};
/// let parser = choice!("foo", "bar", "baz");
/// assert_eq!(Ok(("", "foo")), parser.parse("foo"));
/// assert_eq!(Ok(("", "bar")), parser.parse("bar"));
/// assert_eq!(Err("quiz"), parser.parse("quiz"));
/// ```
#[macro_export]
macro_rules! choice {
    ($parser: expr $(,)?) => {
        $parser
    };
    ($parser: expr, $($rest: expr),+ $(,)?) => {
        or($parser, choice!($($rest),+))
    }
}

// Ordinary parser combinators

/// A parser combinator which takes a parser as input and maps the output using the
/// given transformation function.
///
/// This corresponds to [Result::map]. The value is only mapped if the input parser
/// matches against input.
///
/// # Examples
///
/// ```
/// use helix_parsec::{map, Parser};
/// let parser = map("123", |s| s.parse::<i32>().unwrap());
/// assert_eq!(Ok(("", 123)), parser.parse("123"));
/// assert_eq!(Err("abc"), parser.parse("abc"));
/// ```
pub fn map<'a, P, F, T>(parser: P, map_fn: F) -> impl Parser<'a, Output = T>
where
    P: Parser<'a>,
    F: Fn(P::Output) -> T,
{
    move |input| {
        parser
            .parse(input)
            .map(|(next_input, result)| (next_input, map_fn(result)))
    }
}

/// A parser combinator which succeeds if the given parser matches the input and
/// the given `filter_map_fn` returns `Some`.
///
/// # Examples
///
/// ```
/// use helix_parsec::{filter_map, take_until, Parser};
/// let parser = filter_map(take_until(|c| c == '.'), |s| s.parse::<i32>().ok());
/// assert_eq!(Ok((".456", 123)), parser.parse("123.456"));
/// assert_eq!(Err("abc.def"), parser.parse("abc.def"));
/// ```
pub fn filter_map<'a, P, F, T>(parser: P, filter_map_fn: F) -> impl Parser<'a, Output = T>
where
    P: Parser<'a>,
    F: Fn(P::Output) -> Option<T>,
{
    move |input| match parser.parse(input) {
        Ok((next_input, value)) => match filter_map_fn(value) {
            Some(value) => Ok((next_input, value)),
            None => Err(input),
        },
        Err(_) => Err(input),
    }
}

/// A parser combinator which succeeds if the first given parser matches the input and
/// the second given parse also matches.
///
/// # Examples
///
/// ```
/// use helix_parsec::{reparse_as, take_until, one_or_more, Parser};
/// let parser = reparse_as(take_until(|c| c == '/'), one_or_more("a"));
/// assert_eq!(Ok(("/bb", vec!["a", "a"])), parser.parse("aa/bb"));
/// ```
pub fn reparse_as<'a, P1, P2, T>(parser1: P1, parser2: P2) -> impl Parser<'a, Output = T>
where
    P1: Parser<'a, Output = &'a str>,
    P2: Parser<'a, Output = T>,
{
    filter_map(parser1, move |str| {
        parser2.parse(str).map(|(_, value)| value).ok()
    })
}

/// A parser combinator which only matches the input when the predicate function
/// returns true.
///
/// # Examples
///
/// ```
/// use helix_parsec::{filter, take_until, Parser};
/// let parser = filter(take_until(|c| c == '.'), |s| s == &"123");
/// assert_eq!(Ok((".456", "123")), parser.parse("123.456"));
/// assert_eq!(Err("456.123"), parser.parse("456.123"));
/// ```
pub fn filter<'a, P, F, T>(parser: P, pred_fn: F) -> impl Parser<'a, Output = T>
where
    P: Parser<'a, Output = T>,
    F: Fn(&P::Output) -> bool,
{
    move |input| {
        if let Ok((next_input, value)) = parser.parse(input) {
            if pred_fn(&value) {
                return Ok((next_input, value));
            }
        }
        Err(input)
    }
}

/// A parser combinator which matches either of the input parsers.
///
/// Both parsers must have the same output type. For a variadic form which
/// can take any number of parsers, use `choice!`.
///
/// # Examples
///
/// ```
/// use helix_parsec::{or, Parser};
/// let parser = or("foo", "bar");
/// assert_eq!(Ok(("", "foo")), parser.parse("foo"));
/// assert_eq!(Ok(("", "bar")), parser.parse("bar"));
/// assert_eq!(Err("baz"), parser.parse("baz"));
/// ```
pub fn or<'a, P1, P2, T>(parser1: P1, parser2: P2) -> impl Parser<'a, Output = T>
where
    P1: Parser<'a, Output = T>,
    P2: Parser<'a, Output = T>,
{
    move |input| match parser1.parse(input) {
        ok @ Ok(_) => ok,
        Err(_) => parser2.parse(input),
    }
}

/// A parser combinator which attempts to match the given parser, returning a
/// `None` output value if the parser does not match.
///
/// The parser produced with this combinator always succeeds. If the given parser
/// succeeds, `Some(value)` is returned where `value` is the output of the given
/// parser. Otherwise, `None`.
///
/// # Examples
///
/// ```
/// use helix_parsec::{optional, Parser};
/// let parser = optional("foo");
/// assert_eq!(Ok(("bar", Some("foo"))), parser.parse("foobar"));
/// assert_eq!(Ok(("bar", None)), parser.parse("bar"));
/// ```
pub fn optional<'a, P, T>(parser: P) -> impl Parser<'a, Output = Option<T>>
where
    P: Parser<'a, Output = T>,
{
    move |input| match parser.parse(input) {
        Ok((next_input, value)) => Ok((next_input, Some(value))),
        Err(_) => Ok((input, None)),
    }
}

/// A parser combinator which runs the given parsers in sequence and returns the
/// value of `left` if both are matched.
///
/// This is useful for two-element sequences in which you only want the output
/// value of the `left` parser.
///
/// # Examples
///
/// ```
/// use helix_parsec::{left, Parser};
/// let parser = left("foo", "bar");
/// assert_eq!(Ok(("", "foo")), parser.parse("foobar"));
/// ```
pub fn left<'a, L, R, T>(left: L, right: R) -> impl Parser<'a, Output = T>
where
    L: Parser<'a, Output = T>,
    R: Parser<'a>,
{
    map(seq!(left, right), |(left_value, _)| left_value)
}

/// A parser combinator which runs the given parsers in sequence and returns the
/// value of `right` if both are matched.
///
/// This is useful for two-element sequences in which you only want the output
/// value of the `right` parser.
///
/// # Examples
///
/// ```
/// use helix_parsec::{right, Parser};
/// let parser = right("foo", "bar");
/// assert_eq!(Ok(("", "bar")), parser.parse("foobar"));
/// ```
pub fn right<'a, L, R, T>(left: L, right: R) -> impl Parser<'a, Output = T>
where
    L: Parser<'a>,
    R: Parser<'a, Output = T>,
{
    map(seq!(left, right), |(_, right_value)| right_value)
}

/// A parser combinator which matches the given parser against the input zero or
/// more times.
///
/// This parser always succeeds and returns the empty Vec when it matched zero
/// times.
///
/// # Examples
///
/// ```
/// use helix_parsec::{zero_or_more, Parser};
/// let parser = zero_or_more("a");
/// assert_eq!(Ok(("", vec![])), parser.parse(""));
/// assert_eq!(Ok(("", vec!["a"])), parser.parse("a"));
/// assert_eq!(Ok(("", vec!["a", "a"])), parser.parse("aa"));
/// assert_eq!(Ok(("bb", vec![])), parser.parse("bb"));
/// ```
pub fn zero_or_more<'a, P, T>(parser: P) -> impl Parser<'a, Output = Vec<T>>
where
    P: Parser<'a, Output = T>,
{
    let parser = non_empty(parser);
    move |mut input| {
        let mut values = Vec::new();

        while let Ok((next_input, value)) = parser.parse(input) {
            input = next_input;
            values.push(value);
        }

        Ok((input, values))
    }
}

/// A parser combinator which matches the given parser against the input one or
/// more times.
///
/// This parser combinator acts the same as [zero_or_more] but must match at
/// least once.
///
/// # Examples
///
/// ```
/// use helix_parsec::{one_or_more, Parser};
/// let parser = one_or_more("a");
/// assert_eq!(Err(""), parser.parse(""));
/// assert_eq!(Ok(("", vec!["a"])), parser.parse("a"));
/// assert_eq!(Ok(("", vec!["a", "a"])), parser.parse("aa"));
/// assert_eq!(Err("bb"), parser.parse("bb"));
/// ```
pub fn one_or_more<'a, P, T>(parser: P) -> impl Parser<'a, Output = Vec<T>>
where
    P: Parser<'a, Output = T>,
{
    let parser = non_empty(parser);
    move |mut input| {
        let mut values = Vec::new();

        match parser.parse(input) {
            Ok((next_input, value)) => {
                input = next_input;
                values.push(value);
            }
            Err(err) => return Err(err),
        }

        while let Ok((next_input, value)) = parser.parse(input) {
            input = next_input;
            values.push(value);
        }

        Ok((input, values))
    }
}

/// A parser combinator which matches one or more instances of the given parser
/// interspersed with the separator parser.
///
/// Output values of the separator parser are discarded.
///
/// This is typically used to parse function arguments or list items.
///
/// # Examples
///
/// ```rust
/// use helix_parsec::{sep, Parser};
/// let parser = sep("a", ",");
/// assert_eq!(Ok(("", vec!["a", "a", "a"])), parser.parse("a,a,a"));
/// ```
pub fn sep<'a, P, S, T>(parser: P, separator: S) -> impl Parser<'a, Output = Vec<T>>
where
    P: Parser<'a, Output = T>,
    S: Parser<'a>,
{
    move |mut input| {
        let mut values = Vec::new();

        match parser.parse(input) {
            Ok((next_input, value)) => {
                input = next_input;
                values.push(value);
            }
            Err(err) => return Err(err),
        }

        loop {
            match separator.parse(input) {
                Ok((next_input, _)) => input = next_input,
                Err(_) => break,
            }

            match parser.parse(input) {
                Ok((next_input, value)) => {
                    input = next_input;
                    values.push(value);
                }
                Err(_) => break,
            }
        }

        Ok((input, values))
    }
}

pub fn non_empty<'a, T>(p: impl Parser<'a, Output = T>) -> impl Parser<'a, Output = T> {
    move |input| {
        let (new_input, res) = p.parse(input)?;
        if new_input.len() == input.len() {
            Err(input)
        } else {
            Ok((new_input, res))
        }
    }
}
