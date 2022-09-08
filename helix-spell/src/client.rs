extern crate ispell;
use ispell::{SpellChecker, SpellLauncher};

pub struct Client {
    pub lang: &'static str,
    checker: SpellChecker,
}

impl Client {
    pub fn new() -> Self {
        let lang = "en_US";
        let checker = SpellLauncher::new()
            .aspell()
            .dictionary(lang)
            .launch()
            .unwrap();
        // TODO: instead of unwrap (which panics), figure out proper error handling
        Self { checker, lang }
    }
    pub fn check(&mut self, string: &str) -> Vec<Misspelling> {
        self.checker
            .check(string)
            .unwrap_or(Vec::new())
            .iter()
            .map(|error| Misspelling::new(error.misspelled.to_owned(), error.position))
            .collect()
    }
}

pub struct Misspelling {
    pub word: String,
    pub pos: usize,
}

impl Misspelling {
    pub fn new(word: String, pos: usize) -> Self {
        Self { word, pos }
    }
}
