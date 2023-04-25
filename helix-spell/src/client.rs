//use hunspell_rs::Hunspell;
use std::collections::HashMap;

// dummy placeholder to test behaviour while i figure out how to make hunspell-rs compile
struct Hunspell;

mod hunspell_rs {
    pub enum CheckResult {
        MissingInDictionary,
        CheckOk,
    }
}

impl Hunspell {
    pub fn new(_: &str, _: &str) -> Self {
        Hunspell
    }

    pub fn check(&self, word: &str) -> hunspell_rs::CheckResult {
        if word == "bad" {
            hunspell_rs::CheckResult::MissingInDictionary
        } else {
            hunspell_rs::CheckResult::CheckOk
        }
    }

    fn suggest(&self, _: &str) -> Vec<String> {
        vec!["toto".to_owned()]
    }
}

pub struct Client {
    hunspell: Hunspell,
    suggest_cache: HashMap<String, Vec<String>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
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
