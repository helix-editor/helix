use hunspell_rs::CheckResult;
use hunspell_rs::Hunspell;

pub struct Client {
    lang: String,
    hunspell: Hunspell,
}

impl Client {
    pub fn new() -> Self {
        // TODO: figure out how to determine this
        let lang = "en_US".to_string();
        let hunspell = Hunspell::new(
            "/usr/share/hunspell/en_US.aff",
            "/usr/share/hunspell/en_US.dic",
        );
        Self { hunspell, lang }
    }

    pub fn check_line(&self, word: &str) -> Option<Vec<String>> {
        let result = self.hunspell.check(word);
        match result {
            CheckResult::FoundInDictionary => None,
            CheckResult::MissingInDictionary => {
                let suggestions = self.hunspell.suggest(word);
                Some(suggestions)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Client;

    #[test]
    fn check_line_new() {
        let client = Client::new();
        assert_eq!(client.lang, "en_US");
    }

    #[test]
    fn check_line_correct_spelling() {
        let client = Client::new();
        assert_eq!(client.check_line("yes"), None)
    }

    #[test]
    fn check_line_incorrect_spelling() {
        let client = Client::new();
        let word = "yess";
        let suggestions = Vec::from(
            [
                "yetis", "yeas", "yes", "yeses", "yes's", "yens", "ness", "yest", "less", "cess",
                "mess", "fess", "yeps", "yews", "jess",
            ]
            .map(|word| word.to_string()),
        );
        assert_eq!(client.check_line(word), Some(suggestions))
    }
}
