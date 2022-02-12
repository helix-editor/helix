# Adding Indent Queries

Helix uses tree-sitter to correctly indent new lines. This requires
a tree-sitter grammar and an `indent.scm` query file placed in
`runtime/queries/{language}/indents.scm`. The indentation for a line
is calculated by traversing the syntax tree from the lowest node at the
beginning of the new line. Each of these nodes contributes to the total
indent when it is captured by the query (in what way depends on the name
of the capture).

Note that it matters where these added indents begin. For example,
multiple indent level increases that start on the same line only increase
the total indent level by 1.

## Scopes

Added indents don't always apply to the whole node. For example, in most
cases when a node should be indented, we actually only want everything
except for its first line to be indented. For this, there are several
scopes (more scopes may be added in the future if required):

- `all`:
This scope applies to the whole captured node. This is only different from
`tail` when the captured node is the first node on its line.

- `tail`:
This scope applies to everything except for the first line of the
captured node.

Every capture type has a default scope which should do the right thing
in most situations. When a different scope is required, this can be
changed by using a `#set!` declaration anywhere in the pattern:
```scm
(assignment_expression
  right: (_) @indent
  (#set! "scope" "all"))
```

## Capture Types

- `@indent` (default scope `tail`):
Increase the indent level by 1. Multiple occurences in the same line
don't stack. If there is at least one `@indent` and one `@outdent`
capture on the same line, the indent level isn't changed at all.

- `@outdent` (default scope `all`):
Decrease the indent level by 1. The same rules as for `@indent` apply.
