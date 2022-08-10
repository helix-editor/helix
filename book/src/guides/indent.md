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
Increase the indent level by 1. Multiple occurrences in the same line
don't stack. If there is at least one `@indent` and one `@outdent`
capture on the same line, the indent level isn't changed at all.

- `@outdent` (default scope `all`):
Decrease the indent level by 1. The same rules as for `@indent` apply.

- `@extend-indented`:
Extend the range of this node to the end of the line and to lines that
are indented more than the line that this node starts on. This is useful
for languages like Python, where for the purpose of indentation some nodes
(like functions or classes) should also contain indented lines that follow them.

- `@stop-extend`:
Prevents the first extension of an ancestor of this node. For example, in Python
a return expression always ends the block that it is in. Note that this only prevents
the next extension of one ancestor: If multiple ancestors can be extended (for example
multiple nested conditional blocks in python), only the extension of the innermost
ancestor is prevented.

## Predicates

In some cases, an S-expression cannot express exactly what pattern should be matched.
For that, tree-sitter allows for predicates to appear anywhere within a pattern,
similar to how `#set!` declarations work:
```scm
(some_kind
  (child_kind) @indent
  (#predicate? arg1 arg2 ...)
)
```
The number of arguments depends on the predicate that's used.
Each argument is either a capture (`@name`) or a string (`"some string"`).
The following predicates are supported by tree-sitter:

- `#eq?`/`#not-eq?`:
The first argument (a capture) must/must not be equal to the second argument
(a capture or a string).

- `#match?`/`#not-match?`:
The first argument (a capture) must/must not match the regex given in the
second argument (a string).

Additionally, we support some custom predicates for indent queries:

- `#not-kind-eq?`:
The kind of the first argument (a capture) must not be equal to the second
argument (a string).

- `#same-line?`/`#not-same-line?`:
The captures given by the 2 arguments must/must not start on the same line.
