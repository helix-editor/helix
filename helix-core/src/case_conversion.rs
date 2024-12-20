use crate::Tendril;

// todo: should this be grapheme aware?

/// Converts each character into a different one, with zero context about surrounding characters
pub fn simple_case_conversion(
    text: impl Iterator<Item = char>,
    buf: &mut Tendril,
    transform_char: impl Fn(&char) -> char,
) {
    for c in text {
        buf.push(transform_char(&c))
    }
}

pub fn complex_case_conversion(
    text: impl Iterator<Item = char>,
    buf: &mut Tendril,
    capitalize_first: bool,
    separator: Option<char>,
) {
    let mut should_capitalize_current = capitalize_first;
    let mut prev: Option<char> = None;

    for c in text.skip_while(|ch| ch.is_whitespace()) {
        if c.is_alphanumeric() {
            if let Some(separator) = separator {
                if prev.is_some_and(|p| p != separator)
                    && prev.is_some_and(|p| p.is_lowercase())
                    && c.is_uppercase()
                {
                    buf.push(separator);
                }
            }
            if prev.is_some_and(|p| p.is_lowercase()) && c.is_uppercase() {
                should_capitalize_current = true;
            }
            if should_capitalize_current {
                buf.push(c.to_ascii_uppercase());
                should_capitalize_current = false;
            } else {
                buf.extend(c.to_lowercase());
            }
        } else {
            should_capitalize_current = true;
            if let Some(separator) = separator {
                if prev.is_some_and(|p| p != separator) {
                    buf.push(separator);
                }
            }
        }
        prev = Some(c);
    }

    *buf = buf.trim_end().into();
}

pub fn separator_case_conversion(
    text: impl Iterator<Item = char>,
    buf: &mut Tendril,
    separator: char,
) {
    let mut prev: Option<char> = None;

    for c in text.skip_while(|ch| ch.is_whitespace()) {
        if !c.is_alphanumeric() {
            prev = Some(c);
            continue;
        }

        // "camelCase" => transition at 'l' -> 'C'
        let has_camel_transition = prev.is_some_and(|p| p.is_lowercase()) && c.is_uppercase();
        // "email@somewhere" => transition at 'l' -> '@'
        // first character must not be separator, e.g. @emailSomewhere should not become -email-somewhere
        let has_alphanum_transition = !prev.is_some_and(|p| p.is_alphanumeric()) && !buf.is_empty();

        if has_camel_transition || has_alphanum_transition {
            buf.push(separator);
        }

        buf.push(c.to_ascii_lowercase());

        prev = Some(c);
    }
}

pub fn into_alternate_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    simple_case_conversion(text, buf, |c| {
        if c.is_uppercase() {
            c.to_ascii_lowercase()
        } else if c.is_lowercase() {
            c.to_ascii_uppercase()
        } else {
            *c
        }
    });
}

pub fn into_upper_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    simple_case_conversion(text, buf, char::to_ascii_uppercase);
}

pub fn into_lower_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    simple_case_conversion(text, buf, char::to_ascii_lowercase);
}

pub fn into_kebab_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    separator_case_conversion(text, buf, '-');
}

pub fn into_snake_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    separator_case_conversion(text, buf, '_');
}

pub fn into_title_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    complex_case_conversion(text, buf, true, Some(' '));
}

pub fn into_camel_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    complex_case_conversion(text, buf, false, None);
}

pub fn into_pascal_case(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    complex_case_conversion(text, buf, true, None);
}

fn to_case<I>(text: I, to_case_with: fn(I, &mut Tendril)) -> Tendril
where
    I: Iterator<Item = char>,
{
    let mut res = Tendril::new();
    to_case_with(text, &mut res);
    res
}

pub fn to_camel_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_camel_case)
}

pub fn to_lower_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_lower_case)
}

pub fn to_upper_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_upper_case)
}

pub fn to_pascal_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_pascal_case)
}

pub fn to_alternate_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_alternate_case)
}

pub fn to_title_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_title_case)
}

pub fn to_kebab_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_kebab_case)
}

pub fn to_snake_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, into_snake_case)
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
            assert_eq!(to_lower_case(input.chars()), expected)
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
            assert_eq!(to_upper_case(input.chars()), expected)
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
