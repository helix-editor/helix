# Adding Spellcheck Queries

Helix uses `spellcheck.scm` tree-sitter query files to decide which parts of
a document are spell checked. See [Spell checking](../spell-checking.md) for
info about the feature.

Tree-sitter queries are documented in the tree-sitter online documentation. If
you're writing queries for the first time, be sure to check out the section on
[syntax highlighting queries] and on [query syntax].

Spellcheck queries have two captures:

- `@spell` marks a node whose text should be checked.
- `@nospell` excludes a node (or part of one) from a surrounding `@spell`
  capture.

Queries go in a `runtime/queries/<language>/spellcheck.scm` file. A language
with no query is checked in full, like plain text.

## Checking comments

Most languages inject a shared `comment` grammar into their comments, and that
grammar has a query (`runtime/queries/comment/spellcheck.scm`) capturing
comment text. So you usually do not need a query just to check a language's
comments.

## An example

To check the contents of strings in a language, capture the string's text node:

```scm
(string_content) @spell
```

Use `@nospell` to carve out parts that aren't prose. For example, to check a
string but skip an interpolation or escape inside it:

```scm
(string (string_content) @spell)
(escape_sequence) @nospell
```

The `:tree-sitter-subtree` command shows the syntax tree under the primary
selection and is the easiest way to find the node names to capture.

[syntax highlighting queries]: https://tree-sitter.github.io/tree-sitter/syntax-highlighting#highlights
[query syntax]: https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
