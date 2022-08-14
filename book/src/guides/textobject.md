# Adding Textobject Queries

Textobjects that are language specific ([like functions, classes, etc][textobjects])
require an accompanying tree-sitter grammar and a `textobjects.scm` query file
to work properly. Tree-sitter allows us to query the source code syntax tree
and capture specific parts of it. The queries are written in a lisp dialect.
More information on how to write queries can be found in the [official tree-sitter
documentation][tree-sitter-queries].

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
| `test.inside`      |
| `test.around`      |
| `parameter.inside` |
| `comment.inside`   |
| `comment.around`   |

[Example query files][textobject-examples] can be found in the helix GitHub repository.

## Queries for Textobject Based Navigation

[Tree-sitter based navigation][textobjects-nav] is done using captures in the
following order:

- `object.movement`
- `object.around`
- `object.inside`

For example if a `function.around` capture has been already defined for a language
in it's `textobjects.scm` file, function navigation should also work automatically.
`function.movement` should be defined only if the node captured by `function.around`
doesn't make sense in a navigation context.

[textobjects]: ../usage.md#textobjects
[textobjects-nav]: ../usage.md#tree-sitter-textobject-based-navigation
[tree-sitter-queries]: https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax
[tree-sitter-captures]: https://tree-sitter.github.io/tree-sitter/using-parsers#capturing-nodes
[textobject-examples]: https://github.com/search?q=repo%3Ahelix-editor%2Fhelix+filename%3Atextobjects.scm&type=Code&ref=advsearch&l=&l=
