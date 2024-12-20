use crate::Tendril;

// todo: should this be grapheme aware?

fn to_case<I>(text: I, to_case_with: fn(I, &mut Tendril)) -> Tendril
where
    I: Iterator<Item = char>,
{
    let mut res = Tendril::new();
    to_case_with(text, &mut res);
    res
}

pub fn to_camel_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_camel_case_with)
}

pub fn to_lower_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_lower_case_with)
}

pub fn to_upper_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_upper_case_with)
}

pub fn to_pascal_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_pascal_case_with)
}

pub fn to_alternate_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_alternate_case_with)
}

pub fn to_title_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_title_case_with)
}

pub fn to_kebab_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_kebab_case_with)
}

pub fn to_snake_case(text: impl Iterator<Item = char>) -> Tendril {
    to_case(text, to_snake_case_with)
}

pub fn to_upper_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    for c in text {
        for c in c.to_uppercase() {
            buf.push(c)
        }
    }
}

pub fn to_lower_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    for c in text {
        for c in c.to_lowercase() {
            buf.push(c)
        }
    }
}

pub fn to_alternate_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    for c in text {
        if c.is_uppercase() {
            buf.extend(c.to_lowercase())
        } else if c.is_lowercase() {
            buf.extend(c.to_uppercase())
        } else {
            buf.push(c)
        }
    }
}

pub fn to_title_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    let mut capitalize_next = true;
    let mut prev_is_lowercase = false;

    for c in text {
        if c.is_alphanumeric() {
            if capitalize_next || (prev_is_lowercase && c.is_uppercase()) {
                buf.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                buf.extend(c.to_lowercase());
            }
            prev_is_lowercase = c.is_lowercase();
        } else {
            capitalize_next = true;
            prev_is_lowercase = false;
            buf.push(' ');
        }
    }

    *buf = buf.trim().into();
}

pub fn to_case_with_separator(
    text: impl Iterator<Item = char>,
    buf: &mut Tendril,
    separator: char,
) {
    let mut prev_is_lowercase = false;
    let mut prev_is_alphanumeric = false;

    for c in text {
        if c.is_alphanumeric() {
            if prev_is_lowercase && c.is_uppercase() || !prev_is_alphanumeric && !buf.is_empty() {
                buf.push(separator);
            }

            buf.push(c.to_ascii_lowercase());
            prev_is_lowercase = c.is_lowercase();
            prev_is_alphanumeric = true;
        } else {
            prev_is_lowercase = false;
            prev_is_alphanumeric = false;
        }
    }

    if buf.ends_with(separator) {
        buf.pop();
    }
}

pub fn to_kebab_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    to_case_with_separator(text, buf, '-');
}

pub fn to_snake_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    to_case_with_separator(text, buf, '_');
}

pub fn to_camel_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    to_camel_or_pascal_case_with(text, buf, false);
}

pub fn to_pascal_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    to_camel_or_pascal_case_with(text, buf, true);
}

pub fn to_camel_or_pascal_case_with(
    text: impl Iterator<Item = char>,
    buf: &mut Tendril,
    is_pascal: bool,
) {
    let mut capitalize_next = is_pascal;
    let mut prev_is_lowercase = false;

    for c in text {
        if c.is_alphanumeric() {
            if prev_is_lowercase && c.is_uppercase() {
                capitalize_next = true;
            }
            if capitalize_next {
                buf.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                buf.extend(c.to_lowercase());
            }
            prev_is_lowercase = c.is_lowercase();
        } else {
            capitalize_next = true;
            prev_is_lowercase = false;
        }
    }
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
        // camel_test("   hello world", "helloWorld");
        camel_test("hello\tworld", "helloWorld");
        camel_test("HELLO  WORLD", "helloWorld");
        camel_test("HELLO-world", "helloWorld");
        camel_test("hello  WORLD ", "helloWorld");
        // camel_test("helloWorld", "helloWorld");
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
        // title_test("hello  world", "Hello World");
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
