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
pub fn to_camel_case_with(text: impl Iterator<Item = char>, buf: &mut Tendril) {
    let mut first = true;
    let mut at_word_start = false;

    for c in text {
        if !c.is_alphanumeric() {
            at_word_start = true;
            continue;
        }

        if first {
            buf.extend(c.to_lowercase());
            first = false;
        } else if at_word_start {
            at_word_start = false;
            buf.extend(c.to_uppercase());
        } else {
            buf.extend(c.to_lowercase());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_case_underscore() {
        let result = to_camel_case("otto_botto".chars());
        assert_eq!(result.as_ref() as &str, "ottoBotto");
    }

    #[test]
    fn test_camel_case_uppercase() {
        let result = to_camel_case("OTTO_BOTTO".chars());
        assert_eq!(result.as_ref() as &str, "ottoBotto");
    }

    #[test]
    fn test_camel_case_mixed_case(){
        let result = to_camel_case("OttO_boTTO".chars());
        assert_eq!(result.as_ref() as &str, "ottoBotto");
    }
    
    #[test]
    fn test_camel_case_includes_nums(){
        let result = to_camel_case("Ott0_b0TT0".chars());
        assert_eq!(result.as_ref() as &str, "ott0B0tt0");
    }

    #[test]
    fn test_camel_case_one_word_lower(){
        let result = to_camel_case("otto".chars());
        assert_eq!(result.as_ref() as &str, "otto");
    }

    #[test]
    fn test_camel_case_one_word_upper(){
        let result = to_camel_case("OTTO".chars());
        assert_eq!(result.as_ref() as &str, "otto");
    }

    #[test]
    fn test_camel_case_one_char_lower(){
        let result = to_camel_case("o".chars());
        assert_eq!(result.as_ref() as &str, "o");
    }

    #[test]
    fn test_camel_case_one_char_upper(){
        let result = to_camel_case("O".chars());
        assert_eq!(result.as_ref() as &str, "o");
    }

    #[test]
    fn test_camel_case_many_words_separators(){
        let result = to_camel_case("otto_botto_the_dog".chars());
        assert_eq!(result.as_ref() as &str, "ottoBottoTheDog");
    }

    #[test]
    fn test_camel_case_empty_string(){
        let result = to_camel_case("".chars());
        assert_eq!(result.as_ref() as &str, "");
    }

}
