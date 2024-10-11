use std::iter::Peekable;

const WORDS: [&str; 10] = [
    "TRUE\nFALSE\nTRUE",
    "LEFT\nRIGHT\nLEFT",
    "UP\nDOWN\nUP",
    "FRONT\nBACK\nFRONT",
    "FORWARD\nBACKWARD\nFORWARD",
    "ONE\nTWO\nTHREE\nFOUR\nFIVE\nSIX\nSEVEN\nEIGHT\nNINE\nTEN",
    "NORTH\nSOUTH\nEAST\nWEST\nNORTH",
    "TO_UPPERCASE\nTO_LOWERCASE",
    "TO_ASCII_UPPERCASE\nTO_ASCII_LOWERCASE",
    "PING\nPONG\nPING",
];

/// Increment a string
///
/// Accepted words are hard coded in 'WORDS'.
/// You can create an infinite loop if the first word is the same than the last one.  
/// If there's a match, the next/previous word will be adapted to the current case.
pub fn increment(selected_text: &str, amount: i64) -> Option<String> {
    if !selected_text.is_empty() && selected_text.chars().count() < 20 {
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

/// Consume the it and return the next word if text_up has been found
fn check<T: Iterator<Item = &'static str>>(mut it: Peekable<T>, text_up: &str) -> Option<&str> {
    while let Some(word) = it.next() {
        if let Some(next) = it.peek() {
            if word == text_up {
                return Some(next);
            }
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
