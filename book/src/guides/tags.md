## Adding tags queries

See tree-sitter's documentation on [Code Navigation Systems] for more
background on tags queries.

Helix provides LSP-like features such as document and workspace symbol pickers
out-of-the-box for languages with `tags.scm` queries based on syntax trees. To
be analyzed a language must have a tree-sitter grammar and a `tags.scm` query
file which pattern matches interesting nodes from syntax trees.

Query files should be placed in `runtime/queries/{language}/tags.scm`
when contributing to Helix. You may place these under your local runtime
directory (`~/.config/helix/runtime` in Linux for example) for the sake of
testing.

The following [captures][tree-sitter-captures] are recognized:

| Capture name           |
|---                     |
| `definition.class`     |
| `definition.constant`  |
| `definition.function`  |
| `definition.interface` |
| `definition.macro`     |
| `definition.module`    |
| `definition.section`   |
| `definition.struct`    |
| `definition.type`      |

[Example query files][example-queries] can be found in the Helix GitHub
repository.

[Code Navigation Systems]: https://tree-sitter.github.io/tree-sitter/4-code-navigation.html
[tree-sitter-captures]: https://tree-sitter.github.io/tree-sitter/using-parsers/queries/index.html
[example-queries]: https://github.com/search?q=repo%3Ahelix-editor%2Fhelix+path%3A%2A%2A/tags.scm&type=Code
