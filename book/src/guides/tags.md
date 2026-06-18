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

## Captures

### `@definition.*`

Marks a node as a symbol definition. The following definition captures are
recognized:

| Capture name           |
|---                     |
| `definition.class`     |
| `definition.constant`  |
| `definition.enum`      |
| `definition.field`     |
| `definition.function`  |
| `definition.interface` |
| `definition.macro`     |
| `definition.module`    |
| `definition.section`   |
| `definition.struct`    |
| `definition.type`      |

Captures outside this list are ignored by the symbol pickers, so map a
language-specific construct onto the closest listed kind rather than inventing a
new one.

### `@name`

Marks the name identifier node within a match. `@definition.*` should capture
the entire definition node, and `@name` should capture the name identifier
within that same match:

```scm
(function_definition
  name: (identifier) @name) @definition.function

(class_definition
  name: (identifier) @name) @definition.class
```

### `@reference.*`

Marks a node as a call site or type reference. These are used by workspace
symbol search to locate usages. `@reference.call` and `@reference.class` are
the common variants. As with definitions, `@name` captures the identifier:

```scm
(call
  function: (identifier) @name) @reference.call
```

[Example query files][example-queries] can be found in the Helix GitHub
repository.

[Code Navigation Systems]: https://tree-sitter.github.io/tree-sitter/4-code-navigation.html
[example-queries]: https://github.com/search?q=repo%3Ahelix-editor%2Fhelix+path%3A%2A%2A/tags.scm&type=Code
