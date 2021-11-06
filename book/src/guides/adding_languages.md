# Adding languages

## Submodules

To add a new langauge, you should first add a tree-sitter submodule. To do this, you can run the command
```sh
$ git submodule add -f <repository> helix-syntax/languages/tree-sitter-<name>
```
For example, to add tree-sitter-ocaml you would run
```sh
$ git submodule add -f https://github.com/tree-sitter/tree-sitter-ocaml helix-syntax/languages/tree-sitter-ocaml
```
Make sure the submodule is shallow by doing
```sh
git config -f .gitmodules submodule.helix-syntax/languages/tree-sitter-<name>.shallow true
```

or you can manually add `shallow = true` to `.gitmodules`.

## languages.toml

Next, you need to add the language to the `languages.toml` found in the root of the repository; this `languages.toml` file is included at compilation time, and is distinct from the `language.toml` file in the user's [configuration directory](../configuration.md).

These are the available keys and descriptions for the file.

| Key           | Description                                                   |
| ----          | -----------                                                   |
| name          | The name of the language                                      |
| scope         | A string like `source.js` that identifies the language. Currently, we strive to match the scope names used by popular TextMate grammars and by the Linguist library. Usually `source.<name>` or `text.<name>` in case of markup languages |
| injection-regex | regex pattern that will be tested against a language name in order to determine whether this language should be used for a potential language injection site. [link](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#language-injection) |
| file-types    | The filetypes of the language, for example `["yml", "yaml"]`  |
| roots         | A set of marker files to look for when trying to find the workspace root. For example `Cargo.lock`, `yarn.lock` |
| auto-format   | Whether to autoformat this language when saving               |
| comment-token | The token to use as a comment-token                           |
| indent        | The indent to use. Has sub keys `tab-width` and `unit`            |
| config        | Language server configuration                                 |

## Queries

For a language to have syntax-highlighting and indentation among other things, you have to add queries. Add a directory for your language with the path `runtime/queries/<name>/`. The tree-sitter [website](https://tree-sitter.github.io/tree-sitter/syntax-highlighting#queries) gives more info on how to write queries.

## Common Issues

- If you get errors when building after switching branches, you may have to remove or update tree-sitter submodules. You can update submodules by running
```sh
$ git submodule update --init
```
- Make sure to not use the `--remote` flag. To remove submodules look inside the `.gitmodules` and remove directories that are not present inside of it.

- If a parser is segfaulting or you want to remove the parser, make sure to remove the submodule *and* the compiled parser in `runtime/grammar/<name>.so`

- The indents query is `indents.toml`, *not* `indents.scm`. See [this](https://github.com/helix-editor/helix/issues/114) issue for more information.
