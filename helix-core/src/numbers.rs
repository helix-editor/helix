use std::borrow::Cow;

use ropey::RopeSlice;

use crate::{
    textobject::{textobject_word, TextObject},
    Range,
};

pub struct NumberInfo {
    pub range: Range,
    pub value: i64,
    pub radix: u32,
}

/// Return information about number under cursor if there is one.
pub fn number_at(text: RopeSlice, range: Range) -> Option<NumberInfo> {
    let word_range = textobject_word(text, range, TextObject::Inside, 1, true);
    let word: Cow<str> = text.slice(word_range.from()..word_range.to()).into();
    let (radix, prefixed) = if word.starts_with("0x") {
        (16, true)
    } else if word.starts_with("0o") {
        (8, true)
    } else if word.starts_with("0b") {
        (2, true)
    } else {
        (10, false)
    };

    let number = if prefixed { &word[2..] } else { &word };

    let value = i128::from_str_radix(&number, radix).ok()?;
    if (value.is_positive() && value.leading_zeros() < 64)
        || (value.is_negative() && value.leading_ones() < 64)
    {
        return None;
    }

    let value = value as i64;
    Some(NumberInfo {
        range: word_range,
        value,
        radix,
    })
}
