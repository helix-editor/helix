Syntax symbol pickers
==

This adds two new symbol picker commands that use tree-sitter rather than LSP. We run a new `symbols.scm` query across the file and extract tagged things like function definitions, types, classes, etc. For languages with unambiguous syntax this behaves roughly the same as the LSP symbol picker (`<space>s`). It's less precise though since we don't have semantic info about the language. For example it can easily produce false positives for C/C++ because of preprocessor magic.

The hope is to start introducing LSP-like features for navigation that can work without installing or running a language server. I made these two pickers in particular because I don't like LSP equivalents in ErlangLS - the document symbol picker can take a long time to show up during boot and the workspace symbol picker only searches for module names. The other motivation is to have some navigation features in cases when running a language server is too cumbersome - either to set up or because of resource constraints. For example `clangd` needs a fair amount of setup (`compile_commands.json`) that you might not want to do when quickly reading through a codebase.

GitHub already uses tree-sitter like this to provide [imprecise code navigation](https://docs.github.com/en/repositories/working-with-files/using-files/navigating-code-on-github#about-navigating-code-on-github). It should be possible to find definitions and references as well like `gd` and `gr` - this is left as a follow-up.

This PR also adds commands that either open the LSP symbol picker or the syntax one if a language server is not available. This way you can customize a language to not use the LSP symbol pickers, for example:

```toml
[[language]]
name = "erlang"
language-servers = [{ name = "erlang-ls", except-features = ["document-symbols", "workspace-symbols"] }]
```

and `<space>s` will use the syntax symbol picker, while `<space>s` on a Rust file will still prefer the language server.

---

Outstanding question - how closely should we try to match LSP symbol kind? Not at all? Should we have markup specific symbol kinds? (For example see markdown's `symbols.scm`).

Also this PR needs docs on writing `symbols.scm` queries.
