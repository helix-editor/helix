use std::iter::Peekable;

const MAX_LENGTH_SELECTION: usize = 20;
const WORDS: [&str; 16] = [
    "TRUE\nFALSE",
    "YES\nNO",
    "OK\nNOK",
    "GOOD\nBAD",
    "UP\nDOWN",
    "LEFT\nRIGHT",
    "PING\nPONG",
    "FRONT\nBACK",
    "FORWARD\nBACKWARD",
    "HELLO\nGOOD-BYE",
    "ONE\nTWO\nTHREE\nFOUR\nFIVE\nSIX\nSEVEN\nEIGHT\nNINE\nTEN\nELEVEN\nTWELVE",
    "NORTH\nSOUTH\nEAST\nWEST",
    "U8\nU16\nU32\nU64\nU128\nUSIZE",
    "i8\ni16\ni32\ni64\ni128\niSIZE",
    "TO_UPPERCASE\nTO_LOWERCASE",
    "TO_ASCII_UPPERCASE\nTO_ASCII_LOWERCASE",
];

/// Increment a string
///
/// Accepted words are hard coded in 'WORDS'.
/// If there's a match, the next/previous word will be adapted to the current case.
/// Take the first word if it matches the last one.
pub fn increment(selected_text: &str, amount: i64) -> Option<String> {
    if !selected_text.is_empty() && selected_text.chars().count() < MAX_LENGTH_SELECTION {
        let text_up = selected_text.to_ascii_uppercase();
        for group in WORDS.iter() {
            match amount {
                _ if amount > 0 => {
                    if let Some(right) = check(group.lines().peekable(), &text_up) {
                        return Some(adapt_case(selected_text, right));
                    }
                }
                _ if amount < 0 => {
                    if let Some(right) = check(group.lines().rev().peekable(), &text_up) {
                        return Some(adapt_case(selected_text, right));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Consume the it and return the next word if text_up has been found.
fn check<T: Iterator<Item = &'static str>>(mut it: Peekable<T>, text_up: &str) -> Option<&str> {
    let mut first = "";
    while let Some(word) = it.next() {
        if first.is_empty() {
            first = word;
        }
        if let Some(next) = it.peek() {
            if word == text_up {
                return Some(next);
            }
        } else if word == text_up {
            return Some(first);
        }
    }
    None
}

/// Return the right word adapted to the left's case
fn adapt_case(left: &str, right: &str) -> String {
    if left.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        right.to_uppercase()
    } else if left.starts_with(|c: char| c.is_uppercase()) {
        right
            .char_indices()
            .map(|(i, c)| match i {
                0 => c.to_ascii_uppercase(),
                _ => c.to_ascii_lowercase(),
            })
            .collect()
    } else {
        right.to_lowercase()
    }
}

// cargo test --workspace increment::string::test
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_increment_bool() {
        let tests = [
            ("true", 1, "false"),
            ("False", -1, "True"),
            ("TRUE", 1, "FALSE"),
        ];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }

    #[test]
    fn test_increment_numbers() {
        let tests = [("One", 1, "Two"), ("TWO", -1, "ONE"), ("three", 1, "four")];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }
}
