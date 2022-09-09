extern crate ispell;
use ispell::{IspellError, SpellChecker, SpellLauncher};

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
    pub fn check(&mut self, string: &str) -> Vec<IspellError> {
        self.checker.check(string).unwrap_or(Vec::new())
    }
}

// TODO: use helix_core::Diagnostic to represent a mispelling
// note: Range start is the location of total characters, regardless of "lines"
// thus, we can probably send the entire string to aspell and things should work alright
