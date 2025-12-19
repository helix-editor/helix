use crate::*;

options! {
    struct LanguageConfig {
        /// regex pattern that will be tested against a language name in order to determine whether this language should be used for a potential [language injection][treesitter-language-injection] site.
        #[validator = regex_str_validator()]
        injection_regex: Option<String> = None,
        /// The interpreters from the shebang line, for example `["sh", "bash"]`
        #[read = deref]
        shebangs: List<String> = List::default(),
        /// The token to use as a comment-token
        #[read = deref]
        comment_token: String = "//",
        /// The tree-sitter grammar to use (defaults to the language name)
        grammar: Option<String> = None,
    }

    struct FormatterConfiguration {
        #[read = copy]
        auto_format: bool = true,
        #[name = "formatter.command"]
        formatter_command: Option<String> = None,
        #[name = "formatter.args"]
        #[read = deref]
        formatter_args: List<String> = List::default(),
    }
}
