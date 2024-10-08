# Helix's `lsp-types`

This is a fork of the [`lsp-types`](https://crates.io/crates/lsp-types) crate ([`gluon-lang/lsp-types`](https://github.com/gluon-lang/lsp-types)) taken at version v0.95.1 (commit [3e6daee](https://github.com/gluon-lang/lsp-types/commit/3e6daee771d14db4094a554b8d03e29c310dfcbe)). This fork focuses usability improvements that make the types easier to work with for the Helix codebase. For example the URL type - the `uri` crate at this version of `lsp-types` - will be replaced with a wrapper around a string.
