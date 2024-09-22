/// Toggle a boolean value
pub fn increment(selected_text: &str, _amount: i64) -> Option<String> {
    match selected_text.trim() {
        // Common boolean values
        "true" => Some(String::from("false")),
        "false" => Some(String::from("true")),

        // Python, Haskell
        "True" => Some(String::from("False")),
        "False" => Some(String::from("True")),

        // R, COBOL
        "TRUE" => Some(String::from("FALSE")),
        "FALSE" => Some(String::from("TRUE")),

        // Scheme
        "#t" => Some(String::from("#f")),
        "#f" => Some(String::from("#t")),

        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_boolean_toggles() {
        let tests = [
            // true/false
            ("true", 1, "false"),
            ("true", -1, "false"),
            ("false", 1, "true"),
            ("false", -1, "true"),
            // True/False
            ("True", 1, "False"),
            ("True", -1, "False"),
            ("False", 1, "True"),
            ("False", -1, "True"),
            // TRUE/FALSE
            ("TRUE", 1, "FALSE"),
            ("TRUE", -1, "FALSE"),
            ("FALSE", 1, "TRUE"),
            ("FALSE", -1, "TRUE"),
            // #t/#f
            ("#t", 1, "#f"),
            ("#t", -1, "#f"),
            ("#f", 1, "#t"),
            ("#f", -1, "#t"),
        ];
        for (original, amount, expected) in tests {
            assert_eq!(increment(original, amount).unwrap(), expected);
        }
    }
}
