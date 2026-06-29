use crate::Tendril;

// todo: should this be grapheme aware?

pub fn to_pascal_case(text: impl Iterator<Item = char>) -> Tendril {
    let mut res = Tendril::new();
    to_pascal_case_with(text, &mut res);
    res
}

pub fn to_pascal_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    let mut at_word_start = true;
    for c in text {
        // we don't count _ as a word char here so case conversions work well
        if !c.is_alphanumeric() {
            at_word_start = true;
            continue;
        }
        if at_word_start {
            at_word_start = false;
            buf.extend(c.to_uppercase());
        } else {
            buf.push(c)
        }
    }
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

pub fn to_camel_case(text: impl Iterator<Item = char>) -> Tendril {
    let mut res = Tendril::new();
    to_camel_case_with(text, &mut res);
    res
}
pub fn to_camel_case_with(mut text: impl Iterator<Item = char>, buf: &mut Tendril) {
    // The first word is kept lowercase. Stop consuming as soon as that word
    // ends so the loop below can capitalize the remaining words; leading
    // non-alphanumeric characters are skipped without ending the first word.
    let mut seen_word_char = false;
    for c in &mut text {
        if c.is_alphanumeric() {
            seen_word_char = true;
            buf.extend(c.to_lowercase());
        } else if seen_word_char {
            break;
        }
    }
    let mut at_word_start = true;
    for c in text {
        // we don't count _ as a word char here so case conversions work well
        if !c.is_alphanumeric() {
            at_word_start = true;
            continue;
        }
        if at_word_start {
            at_word_start = false;
            buf.extend(c.to_uppercase());
        } else {
            buf.push(c)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn camel_case_capitalizes_every_word_after_the_first() {
        // Regression: the first word used to consume the whole iterator,
        // leaving the rest un-capitalized, e.g. "foo_bar_baz" -> "foobarbaz".
        let cases = [
            ("foo_bar_baz", "fooBarBaz"),
            ("hello_world", "helloWorld"),
            ("snake_case_here", "snakeCaseHere"),
            ("Foo Bar", "fooBar"),
            // leading, repeated and trailing separators are not words
            ("_leading_sep", "leadingSep"),
            ("foo___bar", "fooBar"),
            ("trailing_", "trailing"),
            // a single word is just lowercased
            ("single", "single"),
            ("alreadyCamel", "alreadycamel"),
            // empty / separator-only input yields nothing
            ("", ""),
            ("___", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(
                &to_camel_case(input.chars())[..],
                expected,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn camel_case_appends_to_an_existing_buffer() {
        // The snippet elaborator reuses one buffer across format items, so the
        // first word must be detected independently of what is already in it.
        let mut buf = Tendril::from("PREFIX_");
        to_camel_case_with("foo_bar".chars(), &mut buf);
        assert_eq!(&buf[..], "PREFIX_fooBar");
    }

    #[test]
    fn pascal_case_capitalizes_every_word() {
        let cases = [
            ("foo_bar_baz", "FooBarBaz"),
            ("hello world", "HelloWorld"),
            ("single", "Single"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                &to_pascal_case(input.chars())[..],
                expected,
                "input: {input:?}"
            );
        }
    }
}
