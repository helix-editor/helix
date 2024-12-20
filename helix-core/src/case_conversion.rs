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
    let mut capitalize_next = capitalize_first;
    let mut prev: Option<char> = None;

    for c in text.skip_while(|ch| ch.is_whitespace()) {
        if c.is_alphanumeric() {
            if prev.is_some_and(|p| p.is_lowercase()) && c.is_uppercase() {
                capitalize_next = true;
            }
            if capitalize_next {
                buf.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                buf.extend(c.to_lowercase());
            }
        } else {
            capitalize_next = true;
            if let Some(separator) = separator {
                if prev.is_some_and(|p| p != separator) {
                    buf.push(separator);
                }
            }
        }
        prev = Some(c);
    }
}

pub fn separator_case_conversion(
    text: impl Iterator<Item = char>,
    buf: &mut Tendril,
    separator: char,
) {
    let mut prev: Option<char> = None;

    for c in text.skip_while(|ch| ch.is_whitespace()) {
        if c.is_alphanumeric() {
            if prev.is_some_and(|p| p.is_lowercase()) && c.is_uppercase()
                || !prev.is_some_and(|p| p.is_alphanumeric()) && !buf.is_empty()
            {
                buf.push(separator);
            }

            buf.push(c.to_ascii_lowercase());
        }
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

    fn case_tester<'a, F>(change_fn: F) -> impl Fn(&'a str, &'a str) + 'a
    where
        F: Fn(std::str::Chars<'a>) -> Tendril + 'a,
    {
        move |input: &str, expected: &str| {
            let transformed = change_fn(input.chars());
            let m = transformed.to_string();
            dbg!(input);
            assert_eq!(m.as_str(), expected)
        }
    }

    #[test]
    fn test_camel_case_conversion() {
        let camel_test = case_tester(to_camel_case);
        camel_test("hello world", "helloWorld");
        camel_test("Hello World", "helloWorld");
        camel_test("hello_world", "helloWorld");
        camel_test("HELLO_WORLD", "helloWorld");
        camel_test("hello-world", "helloWorld");
        camel_test("hello  world", "helloWorld");
        camel_test("   hello world", "helloWorld");
        camel_test("hello\tworld", "helloWorld");
        camel_test("HELLO  WORLD", "helloWorld");
        camel_test("HELLO-world", "helloWorld");
        camel_test("hello  WORLD ", "helloWorld");
        camel_test("helloWorld", "helloWorld");
    }

    #[test]
    fn test_lower_case_conversion() {
        let lower_test = case_tester(to_lower_case);
        lower_test("HelloWorld", "helloworld");
        lower_test("HELLO WORLD", "hello world");
        lower_test("hello_world", "hello_world");
        lower_test("Hello-World", "hello-world");
        lower_test("Hello", "hello");
        lower_test("WORLD", "world");
        lower_test("hello  world", "hello  world");
        lower_test("HELLOworld", "helloworld");
        lower_test("hello-world", "hello-world");
        lower_test("hello_world_here", "hello_world_here");
        lower_test("HELLO_world", "hello_world");
        lower_test("MixEdCaseString", "mixedcasestring");
    }

    #[test]
    fn test_upper_case_conversion() {
        let upper_test = case_tester(to_upper_case);
        upper_test("helloWorld", "HELLOWORLD");
        upper_test("hello world", "HELLO WORLD");
        upper_test("hello_world", "HELLO_WORLD");
        upper_test("Hello-World", "HELLO-WORLD");
        upper_test("Hello", "HELLO");
        upper_test("world", "WORLD");
        upper_test("hello  world", "HELLO  WORLD");
        upper_test("helloworld", "HELLOWORLD");
        upper_test("hello-world", "HELLO-WORLD");
        upper_test("hello_world_here", "HELLO_WORLD_HERE");
        upper_test("hello_WORLD", "HELLO_WORLD");
        upper_test("mixedCaseString", "MIXEDCASESTRING");
    }

    #[test]
    fn test_pascal_case_conversion() {
        let pascal_test = case_tester(to_pascal_case);
        pascal_test("hello world", "HelloWorld");
        pascal_test("Hello World", "HelloWorld");
        pascal_test("hello_world", "HelloWorld");
        pascal_test("HELLO_WORLD", "HelloWorld");
        pascal_test("hello-world", "HelloWorld");
        pascal_test("hello  world", "HelloWorld");
        pascal_test("   hello world", "HelloWorld");
        pascal_test("hello\tworld", "HelloWorld");
        pascal_test("HELLO  WORLD", "HelloWorld");
        pascal_test("HELLO-world", "HelloWorld");
        pascal_test("hello  WORLD ", "HelloWorld");
        pascal_test("helloWorld", "HelloWorld");
    }

    #[test]
    fn test_alternate_case_conversion() {
        let alternate_test = case_tester(to_alternate_case);
        alternate_test("hello world", "HELLO WORLD");
        alternate_test("Hello World", "hELLO wORLD");
        alternate_test("helLo_woRlD", "HELlO_WOrLd");
        alternate_test("HELLO_world", "hello_WORLD");
        alternate_test("hello-world", "HELLO-WORLD");
        alternate_test("Hello-world", "hELLO-WORLD");
        alternate_test("hello", "HELLO");
        alternate_test("HELLO", "hello");
        alternate_test("hello123", "HELLO123");
        alternate_test("hello WORLD", "HELLO world");
        alternate_test("HELLO123 world", "hello123 WORLD");
        alternate_test("world hello", "WORLD HELLO");
    }

    #[test]
    fn test_title_case_conversion() {
        let title_test = case_tester(to_title_case);
        title_test("hello world", "Hello World");
        title_test("Hello World", "Hello World");
        title_test("hello_world", "Hello World");
        title_test("HELLO_WORLD", "Hello World");
        title_test("hello-world", "Hello World");

        title_test("hello  world", "Hello World");

        title_test("   hello world", "Hello World");
        title_test("hello\tworld", "Hello World");
        // title_test("HELLO  WORLD", "Hello World");
        title_test("HELLO-world", "Hello World");
        // title_test("hello  WORLD ", "Hello World");
        // title_test("helloWorld", "Hello World");
    }

    #[test]
    fn test_kebab_case_conversion() {
        let kebab_test = case_tester(to_kebab_case);
        kebab_test("helloWorld", "hello-world");
        kebab_test("HelloWorld", "hello-world");
        kebab_test("hello_world", "hello-world");
        kebab_test("HELLO_WORLD", "hello-world");
        kebab_test("hello-world", "hello-world");
        kebab_test("hello  world", "hello-world");
        kebab_test("hello\tworld", "hello-world");
        kebab_test("HELLO  WORLD", "hello-world");
        kebab_test("HELLO-world", "hello-world");
        kebab_test("hello  WORLD ", "hello-world");
        kebab_test("helloWorld", "hello-world");
        kebab_test("HelloWorld123", "hello-world123");
    }

    #[test]
    fn test_snake_case_conversion() {
        let snake_test = case_tester(to_snake_case);
        snake_test("helloWorld", "hello_world");
        snake_test("HelloWorld", "hello_world");
        snake_test("hello world", "hello_world");
        snake_test("HELLO WORLD", "hello_world");
        snake_test("hello-world", "hello_world");
        snake_test("hello  world", "hello_world");
        snake_test("hello\tworld", "hello_world");
        snake_test("HELLO  WORLD", "hello_world");
        snake_test("HELLO-world", "hello_world");
        snake_test("hello  WORLD ", "hello_world");
        snake_test("helloWorld", "hello_world");
        snake_test("helloWORLD123", "hello_world123");
    }
}
