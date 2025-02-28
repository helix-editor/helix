use std::char::{ToLowercase, ToUppercase};

use crate::Tendril;

// todo: should this be grapheme aware?

pub fn simple_case_conversion(
    chars: impl Iterator<Item = char>,
    buf: &mut Tendril,
    transform_char: impl Fn(&char) -> char,
) {
    *buf = chars.map(|ch| transform_char(&ch)).collect();
}

/// Whether there is a camelCase transition, such as at 'l' -> 'C'
fn has_camel_transition(prev: Option<char>, current: char) -> bool {
    current.is_uppercase() && prev.is_some_and(|ch| ch.is_lowercase())
}

pub fn smart_case_conversion(
    chars: impl Iterator<Item = char>,
    buf: &mut Tendril,
    capitalize_first: bool,
    separator: Option<char>,
) {
    let mut should_capitalize_current = capitalize_first;
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
            should_capitalize_current = true;
            add_separator_if_needed(prev, buf);
            prev = Some(current);
            continue;
        }

        if has_camel_transition(prev, current) {
            add_separator_if_needed(prev, buf);
            should_capitalize_current = true;
        }

        if should_capitalize_current {
            buf.push(current.to_ascii_uppercase());
            should_capitalize_current = false;
        } else {
            buf.push(current.to_ascii_lowercase());
        }

        prev = Some(current);
    }

    *buf = buf.trim_end().into();
}

pub fn separator_case_conversion(
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

        buf.push(current.to_ascii_lowercase());

        prev = Some(current);
    }
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
                let n = if ch.is_some() { 1 } else { 0 };
                (n, Some(n))
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

pub fn into_uppercase(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    *buf = chars.flat_map(char::to_uppercase).collect();
}

pub fn into_lowercase(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    *buf = chars.flat_map(char::to_lowercase).collect();
}

pub fn into_kebab_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    separator_case_conversion(chars, buf, '-');
}

pub fn into_snake_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    separator_case_conversion(chars, buf, '_');
}

pub fn into_title_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    smart_case_conversion(chars, buf, true, Some(' '));
}

pub fn into_camel_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    smart_case_conversion(chars, buf, false, None);
}

pub fn into_pascal_case(chars: impl Iterator<Item = char>, buf: &mut Tendril) {
    smart_case_conversion(chars, buf, true, None);
}

fn to_case<I>(chars: I, into_case: fn(I, &mut Tendril)) -> Tendril
where
    I: Iterator<Item = char>,
{
    let mut res = Tendril::new();
    into_case(chars, &mut res);
    res
}

pub fn to_camel_case(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_camel_case)
}

pub fn to_lowercase(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_lowercase)
}

pub fn to_uppercase(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_uppercase)
}

pub fn to_pascal_case(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_pascal_case)
}

pub fn to_alternate_case(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_alternate_case)
}

pub fn to_title_case(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_title_case)
}

pub fn to_kebab_case(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_kebab_case)
}

pub fn to_snake_case(chars: impl Iterator<Item = char>) -> Tendril {
    to_case(chars, into_snake_case)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case_conversion() {
        let tests = [
            ("hello world", "helloWorld"),
            ("Hello World", "helloWorld"),
            ("hello_world", "helloWorld"),
            ("HELLO_WORLD", "helloWorld"),
            ("hello-world", "helloWorld"),
            ("hello  world", "helloWorld"),
            ("   hello world", "helloWorld"),
            ("hello\tworld", "helloWorld"),
            ("HELLO  WORLD", "helloWorld"),
            ("HELLO-world", "helloWorld"),
            ("hello  WORLD ", "helloWorld"),
            ("helloWorld", "helloWorld"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_camel_case(input.chars()), expected)
        }
    }

    #[test]
    fn test_lower_case_conversion() {
        let tests = [
            ("HelloWorld", "helloworld"),
            ("HELLO WORLD", "hello world"),
            ("hello_world", "hello_world"),
            ("Hello-World", "hello-world"),
            ("Hello", "hello"),
            ("WORLD", "world"),
            ("hello  world", "hello  world"),
            ("HELLOworld", "helloworld"),
            ("hello-world", "hello-world"),
            ("hello_world_here", "hello_world_here"),
            ("HELLO_world", "hello_world"),
            ("MixEdCaseString", "mixedcasestring"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_lowercase(input.chars()), expected)
        }
    }

    #[test]
    fn test_upper_case_conversion() {
        let tests = [
            ("helloWorld", "HELLOWORLD"),
            ("hello world", "HELLO WORLD"),
            ("hello_world", "HELLO_WORLD"),
            ("Hello-World", "HELLO-WORLD"),
            ("Hello", "HELLO"),
            ("world", "WORLD"),
            ("hello  world", "HELLO  WORLD"),
            ("helloworld", "HELLOWORLD"),
            ("hello-world", "HELLO-WORLD"),
            ("hello_world_here", "HELLO_WORLD_HERE"),
            ("hello_WORLD", "HELLO_WORLD"),
            ("mixedCaseString", "MIXEDCASESTRING"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_uppercase(input.chars()), expected)
        }
    }

    #[test]
    fn test_pascal_case_conversion() {
        let tests = [
            ("hello world", "HelloWorld"),
            ("Hello World", "HelloWorld"),
            ("hello_world", "HelloWorld"),
            ("HELLO_WORLD", "HelloWorld"),
            ("hello-world", "HelloWorld"),
            ("hello  world", "HelloWorld"),
            ("   hello world", "HelloWorld"),
            ("hello\tworld", "HelloWorld"),
            ("HELLO  WORLD", "HelloWorld"),
            ("HELLO-world", "HelloWorld"),
            ("hello  WORLD ", "HelloWorld"),
            ("helloWorld", "HelloWorld"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_pascal_case(input.chars()), expected)
        }
    }

    #[test]
    fn test_alternate_case_conversion() {
        let tests = [
            ("hello world", "HELLO WORLD"),
            ("Hello World", "hELLO wORLD"),
            ("helLo_woRlD", "HELlO_WOrLd"),
            ("HELLO_world", "hello_WORLD"),
            ("hello-world", "HELLO-WORLD"),
            ("Hello-world", "hELLO-WORLD"),
            ("hello", "HELLO"),
            ("HELLO", "hello"),
            ("hello123", "HELLO123"),
            ("hello WORLD", "HELLO world"),
            ("HELLO123 world", "hello123 WORLD"),
            ("world hello", "WORLD HELLO"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_alternate_case(input.chars()), expected)
        }
    }

    #[test]
    fn test_title_case_conversion() {
        let tests = [
            ("hello world", "Hello World"),
            ("Hello World", "Hello World"),
            ("hello_world", "Hello World"),
            ("HELLO_WORLD", "Hello World"),
            ("hello-world", "Hello World"),
            ("hello  world", "Hello World"),
            ("   hello world", "Hello World"),
            ("hello\tworld", "Hello World"),
            ("HELLO  WORLD", "Hello World"),
            ("HELLO-world", "Hello World"),
            ("hello  WORLD ", "Hello World"),
            ("helloWorld", "Hello World"),
        ];

        for (input, expected) in tests {
            dbg!(input);
            assert_eq!(to_title_case(input.chars()), expected)
        }
    }

    #[test]
    fn test_kebab_case_conversion() {
        let tests = [
            ("helloWorld", "hello-world"),
            ("HelloWorld", "hello-world"),
            ("hello_world", "hello-world"),
            ("HELLO_WORLD", "hello-world"),
            ("hello-world", "hello-world"),
            ("hello  world", "hello-world"),
            ("hello\tworld", "hello-world"),
            ("HELLO  WORLD", "hello-world"),
            ("HELLO-world", "hello-world"),
            ("hello  WORLD ", "hello-world"),
            ("helloWorld", "hello-world"),
            ("HelloWorld123", "hello-world123"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_kebab_case(input.chars()), expected)
        }
    }

    #[test]
    fn test_snake_case_conversion() {
        let tests = [
            ("helloWorld", "hello_world"),
            ("HelloWorld", "hello_world"),
            ("hello world", "hello_world"),
            ("HELLO WORLD", "hello_world"),
            ("hello-world", "hello_world"),
            ("hello  world", "hello_world"),
            ("hello\tworld", "hello_world"),
            ("HELLO  WORLD", "hello_world"),
            ("HELLO-world", "hello_world"),
            ("hello  WORLD ", "hello_world"),
            ("helloWorld", "hello_world"),
            ("helloWORLD123", "hello_world123"),
        ];

        for (input, expected) in tests {
            assert_eq!(to_snake_case(input.chars()), expected)
        }
    }
}
