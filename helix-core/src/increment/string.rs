const MAX_LENGTH_SELECTION: usize = 20;
const WORDS: [&str; 18] = [
    "TRUE\nFALSE",
    "YES\nNO",
    "OK\nNOK",
    "GOOD\nBAD",
    "UP\nDOWN",
    "LEFT\nRIGHT",
    "PING\nPONG",
    "FRONT\nBACK",
    "FORWARD\nBACKWARD",
    "HELLO\nGOODBYE",
    "ZERO\nONE\nTWO\nTHREE\nFOUR\nFIVE\nSIX\nSEVEN\nEIGHT\nNINE\nTEN\nELEVEN\nTWELVE\nTHIRTEEN\nFOURTEEN\nFIFTEEN",
    "FIRST\nSECOND\nTHIRD\nFOURTH\nSIXTH\nSEVENTH\nEIGHTH\nNINTH\nELEVENTH\nTWELFTH\nTHIRTEENTH\nFOURTEENTH\nFIFTEENTH",
    "1ST\n2ND\n3RD\n4TH\n5TH\n6TH\n7TH\n8TH\n9TH\n10TH\n11TH\n12TH\n13TH\n14TH\n15TH",
    "NORTH\nSOUTH\nEAST\nWEST",
    "U8\nU16\nU32\nU64\nU128\nUSIZE",
    "I8\nI16\nI32\nI64\nI128\nISIZE",
    "TO_UPPERCASE\nTO_LOWERCASE",
    "TO_ASCII_UPPERCASE\nTO_ASCII_LOWERCASE",
];

/// Increment a string
///
/// Accepted words are hard coded in 'WORDS'.
/// If there's a match, the 'amount' next/previous word will be adapted to the current case.
/// Take the first word if it matches the last one.
pub fn increment(selected_text: &str, amount: i64) -> Option<String> {
    if !selected_text.is_empty() && selected_text.chars().count() < MAX_LENGTH_SELECTION {
        let text_up = selected_text.to_ascii_uppercase();

        for group in WORDS.iter() {
            let lines: Vec<&str> = group.lines().collect();

            if let Some(mut pos) = lines.iter().position(|w| **w == text_up) {
                pos = (pos as i64 + amount).rem_euclid(lines.len() as i64) as usize;

                if let Some(new_word) = group.lines().nth(pos) {
                    return Some(adapt_case(selected_text, new_word));
                }
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
            ("TRUE", 2, "TRUE"),
            ("TRUE", 100, "TRUE"),
            ("TRUE", 101, "FALSE"),
            ("False", -100, "False"),
            ("false", 200, "false"),
            ("false", -201, "true"),
        ];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }

    #[test]
    fn test_increment_numbers() {
        let tests = [
            ("One", 1, "Two"),
            ("TWO", -1, "ONE"),
            ("one", 3, "four"),
            ("zero", 4, "four"),
            ("four", -4, "zero"),
            ("1st", 11, "12th"),
            ("2nd", -5, "12th"),
        ];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }

    #[test]
    fn test_increment_data_types() {
        let tests = [
            ("u16", 3, "u128"),
            ("U16", -12, "U16"),
            ("i8", -1, "isize"),
            ("I64", 3, "I8"),
        ];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }
    #[test]
    fn test_increment_special_others() {
        let tests = [
            ("HELLO", 1, "GOODBYE"),
            ("to_uppercase", 3, "to_lowercase"),
            ("South", -3, "East"),
        ];

        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }
}
