# Adding Textobject Queries

Textobjects that are language specific ([like functions, classes, etc][textobjects])
require an accompanying tree-sitter grammar and a `textobjects.scm` query file
to work properly. Tree-sitter allows us to query the source code syntax tree
and capture specific parts of it. The queries are written in a lisp dialect.
More information on how to write queries can be found in the [official tree-sitter
documentation](tree-sitter-queries).

Query files should be placed in `runtime/queries/{language}/textobjects.scm`
when contributing. Note that to test the query files locally you should put
them under your local runtime directory (`~/.config/helix/runtime` on Linux
for example).

The following [captures][tree-sitter-captures] are recognized:

| Capture Name       |
| ---                |
| `function.inside`  |
| `function.around`  |
| `class.inside`     |
| `class.around`     |
| `parameter.inside` |

[Example query files][textobject-examples] can be found in the helix GitHub repository.

[textobjects]: ../usage.md#textobjects
[tree-sitter-queries]: https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax
[tree-sitter-captures]: https://tree-sitter.github.io/tree-sitter/using-parsers#capturing-nodes
[textobject-examples]: https://github.com/search?q=repo%3Ahelix-editor%2Fhelix+filename%3Atextobjects.scm&type=Code&ref=advsearch&l=&l=
