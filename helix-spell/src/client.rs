use hunspell_rs::Hunspell;
use std::collections::HashMap;

pub struct Client {
    hunspell: Hunspell,
    suggest_cache: HashMap<String, Vec<String>>,
}

impl Client {
    pub fn new() -> Self {
        let hunspell = Hunspell::new(
            "/usr/share/hunspell/en_US.aff",
            "/usr/share/hunspell/en_US.dic",
        );
        let suggest_cache = HashMap::new();
        Self {
            hunspell,
            suggest_cache,
        }
    }

    pub fn check(&mut self, word: &str) -> Result<(), Vec<String>> {
        if let hunspell_rs::CheckResult::MissingInDictionary = self.hunspell.check(word) {
            let suggestions = if let Some((_, words)) = self.suggest_cache.get_key_value(word) {
                words
            } else {
                let words = self.hunspell.suggest(word);
                self.suggest_cache.insert(word.to_string(), words);
                self.suggest_cache.get(word).unwrap()
            };
            Err(suggestions.to_vec())
        } else {
            Ok(())
        }
    }
}
