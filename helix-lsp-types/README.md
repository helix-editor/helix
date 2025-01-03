# Helix's `lsp-types`

This is a fork of the [`lsp-types`](https://crates.io/crates/lsp-types) crate ([`gluon-lang/lsp-types`](https://github.com/gluon-lang/lsp-types)) taken at version v0.95.1 (commit [3e6daee](https://github.com/gluon-lang/lsp-types/commit/3e6daee771d14db4094a554b8d03e29c310dfcbe)). This fork focuses on usability improvements that make the types easier to work with for the Helix codebase.

The URL type has been replaced with a newtype wrapper of a `String`. The `lsp-types` crate at the forked version used [`url::Url`](https://docs.rs/url/2.5.0/url/struct.Url.html) which provides conveniences for using URLs according to [the WHATWG URL spec](https://url.spec.whatwg.org). Helix supports a subset of valid URLs, namely the `file://` scheme, so a wrapper around a normal `String` is sufficient. Plus the LSP spec requires URLs to be in [RFC3986](https://tools.ietf.org/html/rfc3986) format instead.
