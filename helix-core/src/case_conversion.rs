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
    let mut prev_is_lowercase = false; // Tracks if the previous character was lowercase
    let mut prev_is_alphanumeric = false; // Tracks if the previous character was alphanumeric

    for c in text {
        if c.is_alphanumeric() {
            if prev_is_lowercase && c.is_uppercase() {
                buf.push(separator);
            }
            if !prev_is_alphanumeric && !buf.is_empty() {
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

pub fn to_camel_case_with(mut text: impl Iterator<Item = char>, buf: &mut Tendril) {
    for c in &mut text {
        if c.is_alphanumeric() {
            buf.extend(c.to_lowercase())
        }
    }
    let mut at_word_start = false;
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
