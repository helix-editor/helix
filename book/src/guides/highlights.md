## Adding highlight queries

`highlights.scm` queries assign a highlight scope (`@function`, `@type`,
`@keyword`, ...) to nodes in the syntax tree; the theme then maps each scope to a
colour. Highlighting is the one query file that every language needs.

Query files should be placed in `runtime/queries/{language}/highlights.scm` when
contributing to Helix.

## Scopes

The full list of highlight scopes, and what each is for, is documented on the
[themes page]. Match the most specific scope that fits the node — for example a
method call is `@function.method` while a plain field access is
`@variable.other.member`.

A query file may reuse another language's with `; inherits: <lang>` on the first
line (for example `tsx` inherits `typescript`, which inherits `ecma`). An
inherited file is compiled against *each* inheriting grammar, so every capture
must be valid there too.

## Precedence

Two rules decide which capture wins when more than one matches the same text:

1. **Same span: last match wins.** Among captures covering the same bytes, the
   pattern that appears later in the file wins. Put a generic rule *before* the
   specific one that should override it.
2. **Nested nodes: innermost wins.** When a parent and a child node both cover
   the text, the child's capture wins, regardless of file order.

A common consequence of rule 2: capture the *leaf* you mean. A call captured on
a wrapping node loses to a base `(identifier) @variable` on the inner identifier,
so put `@function` on the identifier itself.

Where the grammar can't distinguish a scope, casing is often used as a heuristic:

```scm
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_]*$"))
```

## Testing

`cargo xtask query-check [language]` confirms the queries are valid against the
grammar. `cargo xtask highlight-check [language]` runs the real highlighter over
the fixtures in `tests/query/highlights/<language-id>/<name>.<ext>`, where a
caret comment line (`// ^ @capture`) asserts the winning scope at the column
above it; this catches the precedence mistakes that `query-check` cannot see.
`cargo xtask highlight-check --dump <language> <file>` prints the winning capture
per span for an arbitrary file.

[themes page]: https://docs.helix-editor.com/themes.html#scopes
