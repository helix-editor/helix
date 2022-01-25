# Adding languages

## Grammars manifest

To add a new language, add a new line to `scripts/revisions.txt` with the
name of the remote and the revision. Then run

```sh
./scripts/grammars sync
```

To download the new grammar.

## languages.toml

Next, you need to add the language to the [`languages.toml`][languages.toml] found in the root of
the repository; this `languages.toml` file is included at compilation time, and
is distinct from the `language.toml` file in the user's [configuration
directory](../configuration.md).

These are the available keys and descriptions for the file.

| Key                 | Description                                                   |
| ----                | -----------                                                   |
| name                | The name of the language                                      |
| scope               | A string like `source.js` that identifies the language. Currently, we strive to match the scope names used by popular TextMate grammars and by the Linguist library. Usually `source.<name>` or `text.<name>` in case of markup languages |
| injection-regex     | regex pattern that will be tested against a language name in order to determine whether this language should be used for a potential [language injection][treesitter-language-injection] site. |
| file-types          | The filetypes of the language, for example `["yml", "yaml"]`  |
| shebangs            | The interpreters from the shebang line, for example `["sh", "bash"]` |
| roots               | A set of marker files to look for when trying to find the workspace root. For example `Cargo.lock`, `yarn.lock` |
| auto-format         | Whether to autoformat this language when saving               |
| diagnostic-severity | Minimal severity of diagnostic for it to be displayed. (Allowed values: `Error`, `Warning`, `Info`, `Hint`) |
| comment-token       | The token to use as a comment-token                           |
| indent              | The indent to use. Has sub keys `tab-width` and `unit`        |
| config              | Language server configuration                                 |

## Queries

For a language to have syntax-highlighting and indentation among
other things, you have to add queries. Add a directory for your
language with the path `runtime/queries/<name>/`. The tree-sitter
[website](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#queries)
gives more info on how to write queries.

> NOTE: When evaluating queries, the first matching query takes
precedence, which is different from other editors like neovim where
the last matching query supercedes the ones before it. See
[this issue][neovim-query-precedence] for an example.

## Common Issues

- If you get errors when building after switching branches, you may have to sync your grammars. You can sync grammars by running
    ```sh
    ./scripts/grammars sync
    ```
- If a parser is segfaulting or you want to remove the parser, make sure to remove the grammar repository *and* the compiled parser in `runtime/grammar/<name>.so`. You can remove the grammar repository by running
    ```sh
    ./scripts/grammars clean
    ```
- The indents query is `indents.toml`, *not* `indents.scm`. See [this](https://github.com/helix-editor/helix/issues/114) issue for more information.

[treesitter-language-injection]: https://tree-sitter.github.io/tree-sitter/syntax-highlighting#language-injection
[languages.toml]: https://github.com/helix-editor/helix/blob/master/languages.toml
[neovim-query-precedence]: https://github.com/helix-editor/helix/pull/1170#issuecomment-997294090
