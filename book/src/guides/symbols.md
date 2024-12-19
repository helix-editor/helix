## Adding symbols queries

Helix provides LSP-like features such as document and workspace symbol pickers
which extract symbols only from the syntax of source files. To be analyzed a
language must have a tree-sitter grammar and a `symbols.scm` query file which
pattern matches symbols.

Query files should be placed in `runtime/queries/{language}/symbols.scm`
when contributing to Helix. You may place these under your local runtime
directory (`~/.config/helix/runtime` in Linux for example) for the sake of
testing.

The following [captures][tree-sitter-captures] are recognized:

| Capture name           |
|---                     |
| `definition.function`  |
| `definition.macro`     |
| `definition.module`    |
| `definition.constant`  |
| `definition.struct`    |
| `definition.interface` |
| `definition.type`      |
| `definition.class`     |

[Example query files][example-queries] can be found in the Helix GitHub
repository.

[tree-sitter-captures]: https://tree-sitter.github.io/tree-sitter/using-parsers#capturing-nodes
[example-queries]: https://github.com/search?q=repo%3Ahelix-editor%2Fhelix+path%3A%2A%2A/symbols.scm&type=Code&ref=advsearch&l=&l=
