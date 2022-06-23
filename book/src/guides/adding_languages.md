# Adding languages

## Language configuration

To add a new language, you need to add a `[[language]]` entry to the
`languages.toml` (see the [language configuration section]).

When adding a new language or Language Server configuration for an existing
language, run `cargo xtask docgen` to add the new configuration to the
[Language Support][lang-support] docs before creating a pull request.
When adding a Language Server configuration, be sure to update the
[Language Server Wiki][install-lsp-wiki] with installation notes.

## Grammar configuration

If a tree-sitter grammar is available for the language, add a new `[[grammar]]`
entry to `languages.toml`.

You may use the `source.path` key rather than `source.git` with an absolute path
to a locally available grammar for testing, but switch to `source.git` before
submitting a pull request.

## Queries

For a language to have syntax-highlighting and indentation among
other things, you have to add queries. Add a directory for your
language with the path `runtime/queries/<name>/`. The tree-sitter
[website](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#queries)
gives more info on how to write queries.

> NOTE: When evaluating queries, the first matching query takes
precedence, which is different from other editors like neovim where
the last matching query supersedes the ones before it. See
[this issue][neovim-query-precedence] for an example.

## Common Issues

- If you get errors when running after switching branches, you may have to update the tree-sitter grammars. Run `hx --grammar fetch` to fetch the grammars and `hx --grammar build` to build any out-of-date grammars.

- If a parser is segfaulting or you want to remove the parser, make sure to remove the compiled parser in `runtime/grammar/<name>.so`

[language configuration section]: ../languages.md
[neovim-query-precedence]: https://github.com/helix-editor/helix/pull/1170#issuecomment-997294090
[install-lsp-wiki]: https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers
[lang-support]: ../lang-support.md
