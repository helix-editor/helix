# Adding languages

## Language configuration

To add a new language, you need to add a `language` entry to the
[`languages.toml`][languages.toml] found in the root of the repository;
this `languages.toml` file is included at compilation time, and is
distinct from the `languages.toml` file in the user's [configuration
directory](../configuration.md).

```toml
[[language]]
name = "mylang"
scope = "scope.mylang"
injection-regex = "^mylang$"
file-types = ["mylang", "myl"]
comment-token = "#"
indent = { tab-width = 2, unit = "  " }
language-server = { command = "mylang-lsp", args = ["--stdio"] }
```

These are the available keys and descriptions for the file.

| Key                   | Description                                                   |
| ----                  | -----------                                                   |
| `name`                | The name of the language                                      |
| `scope`               | A string like `source.js` that identifies the language. Currently, we strive to match the scope names used by popular TextMate grammars and by the Linguist library. Usually `source.<name>` or `text.<name>` in case of markup languages |
| `injection-regex`     | regex pattern that will be tested against a language name in order to determine whether this language should be used for a potential [language injection][treesitter-language-injection] site. |
| `file-types`          | The filetypes of the language, for example `["yml", "yaml"]`. Extensions and full file names are supported.  |
| `shebangs`            | The interpreters from the shebang line, for example `["sh", "bash"]` |
| `roots`               | A set of marker files to look for when trying to find the workspace root. For example `Cargo.lock`, `yarn.lock` |
| `auto-format`         | Whether to autoformat this language when saving               |
| `diagnostic-severity` | Minimal severity of diagnostic for it to be displayed. (Allowed values: `Error`, `Warning`, `Info`, `Hint`) |
| `comment-token`       | The token to use as a comment-token                           |
| `indent`              | The indent to use. Has sub keys `tab-width` and `unit`        |
| `language-server`     | The Language Server to run. Has sub keys `command` and `args` |
| `config`              | Language Server configuration                                 |
| `grammar`             | The tree-sitter grammar to use (defaults to the value of `name`) |

When adding a new language or Language Server configuration for an existing
language, run `cargo xtask docgen` to add the new configuration to the
[Language Support][lang-support] docs before creating a pull request.
When adding a Language Server configuration, be sure to update the
[Language Server Wiki][install-lsp-wiki] with installation notes.

## Grammar configuration

If a tree-sitter grammar is available for the language, add a new `grammar`
entry to `languages.toml`.

```toml
[[grammar]]
name = "mylang"
source = { git = "https://github.com/example/mylang", rev = "a250c4582510ff34767ec3b7dcdd3c24e8c8aa68" }
```

Grammar configuration takes these keys:

| Key      | Description                                                              |
| ---      | -----------                                                              |
| `name`   | The name of the tree-sitter grammar                                      |
| `source` | The method of fetching the grammar - a table with a schema defined below |

Where `source` is a table with either these keys when using a grammar from a
git repository:

| Key    | Description                                               |
| ---    | -----------                                               |
| `git`  | A git remote URL from which the grammar should be cloned  |
| `rev`  | The revision (commit hash or tag) which should be fetched |
| `subpath` | A path within the grammar directory which should be built. Some grammar repositories host multiple grammars (for example `tree-sitter-typescript` and `tree-sitter-ocaml`) in subdirectories. This key is used to point `hx --grammar build` to the correct path for compilation. When omitted, the root of repository is used |

Or a `path` key with an absolute path to a locally available grammar directory.

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

- The indents query is `indents.toml`, *not* `indents.scm`. See [this](https://github.com/helix-editor/helix/issues/114) issue for more information.

[treesitter-language-injection]: https://tree-sitter.github.io/tree-sitter/syntax-highlighting#language-injection
[languages.toml]: https://github.com/helix-editor/helix/blob/master/languages.toml
[neovim-query-precedence]: https://github.com/helix-editor/helix/pull/1170#issuecomment-997294090
[install-lsp-wiki]: https://github.com/helix-editor/helix/wiki/How-to-install-the-default-language-servers
[lang-support]: ../lang-support.md
