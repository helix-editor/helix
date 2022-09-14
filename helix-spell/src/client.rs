extern crate ispell;
use ispell::{IspellError, SpellChecker, SpellLauncher};

pub struct Client {
    pub lang: &'static str,
    checker: SpellChecker,
}

impl Client {
    pub fn new() -> Self {
        // TODO: accept lang, mode as an argument, configurable by the user
        let lang = "en_US";
        let checker = SpellLauncher::new()
            .hunspell()
            // .aspell()
            .dictionary(lang)
            .launch()
            // TODO: instead of unwrap (which panics), figure out proper error handling
            .unwrap();
        Self { checker, lang }
    }
    pub fn check(&mut self, string: &str) -> Vec<IspellError> {
        self.checker.check(string).unwrap_or(Vec::new())
    }
}

// TODO: expose the ability to add words to a user's dictionary, which the ispell crate supports
