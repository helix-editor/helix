pub mod client;

pub use client::Client;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client() {
        let client = Client::new();
        assert_eq!(client.lang, "en_US");
    }

    #[test]
    fn misspelled_word() {
        let mut client = Client::new();
        let word = "This sentence contains a misssspelled word";
        let errors = client.check(word);
        let error = errors.first().unwrap();
        assert_eq!(error.word, "misssspelled");
        assert_eq!(error.pos, 25);
    }

    #[test]
    fn no_misspelled_word() {
        let mut client = Client::new();
        let word = "This sentence does not contain a misspelled word";
        let errors = client.check(word);
        assert_eq!(errors.len(), 0);
    }
}
