use std::char::{ToLowercase, ToUppercase};

use crate::Tendril;

// todo: should this be grapheme aware?

/// Whether there is a camelCase transition, such as at 'l' -> 'C'
fn has_camel_transition(prev: Option<char>, current: char) -> bool {
    current.is_uppercase() && prev.is_some_and(|ch| ch.is_lowercase())
}

/// In-place conversion into `UPPERCASE`
pub fn into_uppercase(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    *buf = chars.flat_map(char::to_uppercase).collect();
}

/// In-place conversion into `lowercase`
pub fn into_lowercase(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    *buf = chars.flat_map(char::to_lowercase).collect();
}

/// Change case of a character iterator and put the result into the `buf`.
///
/// # Arguments
///
/// - `separator`: Choose what character to insert between the words
fn case_converter_with_separator(
    chars: impl Iterator<Item = char>,
    buf: &mut Tendril,
    separator: char,
) {
    let mut prev = None;

    for current in chars.skip_while(|ch| ch.is_whitespace()) {
        if !current.is_alphanumeric() {
            prev = Some(current);
            continue;
        }

        // "email@somewhere" => transition at 'l' -> '@'
        // first character must not be separator, e.g. @emailSomewhere should not become -email-somewhere
        let has_alphanum_transition = !prev.is_some_and(|p| p.is_alphanumeric()) && !buf.is_empty();

        if has_camel_transition(prev, current) || has_alphanum_transition {
            buf.push(separator);
        }

        buf.extend(current.to_lowercase());

        prev = Some(current);
    }
}

/// In-place conversion into `kebab-case`
pub fn into_kebab_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    case_converter_with_separator(chars, buf, '-');
}

/// In-place conversion into `PascalCase`
pub fn into_snake_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    case_converter_with_separator(chars, buf, '_');
}

// todo: should this be grapheme aware?

/// Choose how words are capitalized
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum CapitalizeWords {
    /// # Examples
    ///
    /// - oneTwoThree
    /// - one-Two-Three
    /// - one Two Three
    AllButFirst,
    /// # Examples
    ///
    /// - OneTwoThree
    /// - One-Two-Three
    /// - One Two Three
    All,
    /// # Examples
    ///
    /// - Onetwothree
    /// - One-two-three
    /// - One two three
    First,
}

/// Change case of a character iterator and put the result into the `buf`.
///
/// # Arguments
///
/// - `capitalize`: Choose how to capitalize the output
/// - `separator`: Character to insert between the words
fn case_converter_with_capitalization_and_separator(
    chars: impl Iterator<Item = char>,
    buf: &mut Tendril,
    capitalize: CapitalizeWords,
    separator: Option<char>,
) {
    let mut should_capitalize_current = capitalize != CapitalizeWords::AllButFirst;
    let mut prev = None;

    let add_separator_if_needed = |prev: Option<char>, buf: &mut Tendril| {
        if let Some(separator) = separator {
            // We do not want to add a separator when the previous char is not a separator
            // For example, snake__case is invalid
            if prev.is_some_and(|ch| ch != separator) {
                buf.push(separator);
            }
        }
    };

    for current in chars.skip_while(|ch| ch.is_whitespace()) {
        if !current.is_alphanumeric() {
            should_capitalize_current = capitalize != CapitalizeWords::First;
            add_separator_if_needed(prev, buf);
            prev = Some(current);
            continue;
        }

        if has_camel_transition(prev, current) {
            add_separator_if_needed(prev, buf);
            should_capitalize_current = capitalize != CapitalizeWords::First;
        }

        if should_capitalize_current {
            buf.extend(current.to_uppercase());
            should_capitalize_current = false;
        } else {
            buf.extend(current.to_lowercase());
        }

        prev = Some(current);
    }

    *buf = buf.trim_end().into();
}

/// In-place conversion into `Title Case`
pub fn into_title_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    case_converter_with_capitalization_and_separator(chars, buf, CapitalizeWords::All, Some(' '));
}

/// In-place conversion into `Sentence case`
pub fn into_sentence_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    case_converter_with_capitalization_and_separator(chars, buf, CapitalizeWords::First, Some(' '));
}

/// In-place conversion into `camelCase`
pub fn into_camel_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    case_converter_with_capitalization_and_separator(
        chars,
        buf,
        CapitalizeWords::AllButFirst,
        None,
    );
}

/// In-place conversion into `PascalCase`
pub fn into_pascal_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    case_converter_with_capitalization_and_separator(chars, buf, CapitalizeWords::All, None);
}

enum AlternateCase {
    Upper(ToUppercase),
    Lower(ToLowercase),
    Keep(Option<char>),
}

impl Iterator for AlternateCase {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            AlternateCase::Upper(upper) => upper.next(),
            AlternateCase::Lower(lower) => lower.next(),
            AlternateCase::Keep(ch) => ch.take(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            AlternateCase::Upper(upper) => upper.size_hint(),
            AlternateCase::Lower(lower) => lower.size_hint(),
            AlternateCase::Keep(ch) => {
                let size = ch.is_some() as usize;
                (size, Some(size))
            }
        }
    }
}

impl ExactSizeIterator for AlternateCase {}

pub fn into_alternate_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    *buf = chars
        .flat_map(|ch| {
            if ch.is_lowercase() {
                AlternateCase::Upper(ch.to_uppercase())
            } else if ch.is_uppercase() {
                AlternateCase::Lower(ch.to_lowercase())
            } else {
                AlternateCase::Keep(Some(ch))
            }
        })
        .collect();
}

/// Create functional versions of the "into_*" case functions that take a `&mut Tendril`
macro_rules! to_case {
    ($($into_case:ident => $to_case:ident)*) => {
        $(
            pub fn $to_case(chars: impl Iterator<Item = char>) -> Tendril {
                let mut res = Tendril::new();
                $into_case(chars, &mut res);
                res
            }
        )*
    };
}

to_case! {
    into_camel_case => to_camel_case
    into_lowercase => to_lowercase
    into_uppercase => to_uppercase
    into_pascal_case => to_pascal_case
    into_alternate_case => to_alternate_case
    into_title_case => to_title_case
    into_kebab_case => to_kebab_case
    into_snake_case => to_snake_case
    into_sentence_case => to_sentence_case
}

#[cfg(test)]
mod tests {
    macro_rules! test_case_converters {
        ($($case_converter:ident: $($input:literal -> $expected:literal)*)*) => {
            $(
                #[test]
                fn $case_converter() {
                    for (input, expected) in [ $(($input, $expected),)* ] {
                        assert_eq!(super::$case_converter(input.chars()), expected, "{input}");
                    }
                }
            )*
        }
    }

    test_case_converters! {
        to_camel_case:
            "hello world" -> "helloWorld"
            "Hello World" -> "helloWorld"
            "hello_world" -> "helloWorld"
            "HELLO_WORLD" -> "helloWorld"
            "hello-world" -> "helloWorld"
            "hello  world" -> "helloWorld"
            "   hello world" -> "helloWorld"
            "hello\tworld" -> "helloWorld"
            "HELLO  WORLD" -> "helloWorld"
            "HELLO-world" -> "helloWorld"
            "hello  WORLD " -> "helloWorld"
            "helloWorld" -> "helloWorld"

        to_pascal_case:
            "hello world" -> "HelloWorld"
            "Hello World" -> "HelloWorld"
            "hello_world" -> "HelloWorld"
            "HELLO_WORLD" -> "HelloWorld"
            "hello-world" -> "HelloWorld"
            "hello  world" -> "HelloWorld"
            "   hello world" -> "HelloWorld"
            "hello\tworld" -> "HelloWorld"
            "HELLO  WORLD" -> "HelloWorld"
            "HELLO-world" -> "HelloWorld"
            "hello  WORLD " -> "HelloWorld"
            "helloWorld" -> "HelloWorld"

        to_snake_case:
            "helloWorld" -> "hello_world"
            "HelloWorld" -> "hello_world"
            "hello world" -> "hello_world"
            "HELLO WORLD" -> "hello_world"
            "hello-world" -> "hello_world"
            "hello  world" -> "hello_world"
            "hello\tworld" -> "hello_world"
            "HELLO  WORLD" -> "hello_world"
            "HELLO-world" -> "hello_world"
            "hello  WORLD " -> "hello_world"
            "helloWorld" -> "hello_world"
            "helloWORLD123" -> "hello_world123"

        to_kebab_case:
            "helloWorld" -> "hello-world"
            "HelloWorld" -> "hello-world"
            "hello_world" -> "hello-world"
            "HELLO_WORLD" -> "hello-world"
            "hello-world" -> "hello-world"
            "hello  world" -> "hello-world"
            "hello\tworld" -> "hello-world"
            "HELLO  WORLD" -> "hello-world"
            "HELLO-world" -> "hello-world"
            "hello  WORLD " -> "hello-world"
            "helloWorld" -> "hello-world"
            "HelloWorld123" -> "hello-world123"

        to_title_case:
            "hello world" -> "Hello World"
            "hello world again" -> "Hello World Again"
            "Hello World" -> "Hello World"
            "hello_world" -> "Hello World"
            "HELLO_WORLD" -> "Hello World"
            "hello-world" -> "Hello World"
            "hello  world" -> "Hello World"
            "   hello world" -> "Hello World"
            "hello\tworld" -> "Hello World"
            "HELLO  WORLD" -> "Hello World"
            "HELLO-world" -> "Hello World"
            "hello  WORLD " -> "Hello World"
            "helloWorld" -> "Hello World"

        to_sentence_case:
            "hello world" -> "Hello world"
            "hello world again" -> "Hello world again"
            "Hello World" -> "Hello world"
            "hello_world" -> "Hello world"
            "HELLO_WORLD" -> "Hello world"
            "hello-world" -> "Hello world"
            "hello  world" -> "Hello world"
            "   hello world" -> "Hello world"
            "hello\tworld" -> "Hello world"
            "HELLO  WORLD" -> "Hello world"
            "HELLO-world" -> "Hello world"
            "hello  WORLD " -> "Hello world"
            "helloWorld" -> "Hello world"

        to_alternate_case:
            "hello world" -> "HELLO WORLD"
            "Hello World" -> "hELLO wORLD"
            "helLo_woRlD" -> "HELlO_WOrLd"
            "HELLO_world" -> "hello_WORLD"
            "hello-world" -> "HELLO-WORLD"
            "Hello-world" -> "hELLO-WORLD"
            "hello" -> "HELLO"
            "HELLO" -> "hello"
            "hello123" -> "HELLO123"
            "hello WORLD" -> "HELLO world"
            "HELLO123 world" -> "hello123 WORLD"
            "world hello" -> "WORLD HELLO"

        to_uppercase:
            "helloWorld" -> "HELLOWORLD"
            "hello world" -> "HELLO WORLD"
            "hello_world" -> "HELLO_WORLD"
            "Hello-World" -> "HELLO-WORLD"
            "Hello" -> "HELLO"
            "world" -> "WORLD"
            "hello  world" -> "HELLO  WORLD"
            "helloworld" -> "HELLOWORLD"
            "hello-world" -> "HELLO-WORLD"
            "hello_world_here" -> "HELLO_WORLD_HERE"
            "hello_WORLD" -> "HELLO_WORLD"
            "mixedCaseString" -> "MIXEDCASESTRING"

        to_lowercase:
            "HelloWorld" -> "helloworld"
            "HELLO WORLD" -> "hello world"
            "hello_world" -> "hello_world"
            "Hello-World" -> "hello-world"
            "Hello" -> "hello"
            "WORLD" -> "world"
            "hello  world" -> "hello  world"
            "HELLOworld" -> "helloworld"
            "hello-world" -> "hello-world"
            "hello_world_here" -> "hello_world_here"
            "HELLO_world" -> "hello_world"
            "MixEdCaseString" -> "mixedcasestring"
    }
}
