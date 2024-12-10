# Adding context queries

Helix uses tree-sitter to filter out specific scopes in which said scope may exceed the current
editor view, but which may be important for the developer to know.
These context require an accompanying tree-sitter grammar and a `context.scm` query file
to work properly.
Query files should be placed in `runtime/queries/{language}/context.scm`
when contributing to Helix. Note that to test the query files locally you should put
them under your local runtime directory (`~/.config/helix/runtime` on Linux for example).

The following [captures][tree-sitter-captures] are recognized:

| Capture Name     |
| ---              |
| `context`        |
| `context.params` |

[Example query files][context-examples] can be found in the helix GitHub repository.

## Queries for the sticky-context feature

All nodes that have a scope, should be captured with `context`, as an example a basic class.
The `context.params` is a capture for all the function parameters. 

[tree-sitter-captures]: https://tree-sitter.github.io/tree-sitter/using-parsers#capturing-nodes
[context-examples]: https://github.com/search?q=repo%3Ahelix-editor%2Fhelix+filename%3Acontext.scm&type=Code&ref=advsearch&l=&l=
