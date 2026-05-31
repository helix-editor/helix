## Adding Locals Queries

`locals.scm` queries teach Helix about variable scopes and definitions so that
local variables can be highlighted distinctly from global ones. When a
`@local.reference` identifier is resolved to a `@local.definition`, it inherits
the highlight class of that definition rather than the class assigned by
`highlights.scm`.

Query files should be placed in `runtime/queries/{language}/locals.scm` when
contributing to Helix.

## Captures

### `@local.scope`

Marks a node as a scope boundary. Definitions are only visible to references
that appear inside the same scope or a nested one. Typical scope nodes are
function bodies and blocks.

```scm
[
  (function_definition)
  (block)
] @local.scope
```

### `@local.definition.*`

Marks a name node as introducing a local symbol. The suffix after
`@local.definition.` becomes the highlight class applied to any reference that
resolves to this definition. For example, `@local.definition.variable.parameter`
causes matching references to be highlighted as `variable.parameter`.

```scm
(function_item
  (parameters
    (parameter
      pattern: (identifier) @local.definition.variable.parameter)))
```

Common suffixes mirror the highlight classes in `highlights.scm`:
`variable`, `variable.parameter`, `variable.builtin`, `variable.mutable`,
`function`, `namespace`, `type`, `constant`, etc.

### `@local.reference`

Marks an identifier node as a potential reference to a local definition.
Helix searches enclosing scopes for a matching definition and, if found,
highlights the reference with that definition's class instead.

```scm
(identifier) @local.reference
```

## Discard captures

Any capture in `locals.scm` that is not `@local.scope`, `@local.reference`, or
`@local.definition.*` acts as a discard. It prevents a `@local.reference` from
being resolved at that node without affecting the `highlights.scm` result. This
is useful for identifier nodes that look like references but should not be
treated as variable references, for example keyword argument names or struct
field names in struct literals.

```scm
; Keyword argument names in a call are not variable references.
(keyword_argument
  name: (identifier) @_)
```

Later patterns in a query file have higher precedence than earlier ones. A
discard pattern must appear after the `@local.reference` pattern it is intended
to override. Placing it later ensures it takes precedence and cancels the
reference resolution for nodes it matches.

The convention is to use a capture name beginning with an underscore (e.g. `@_`,
`@_keyword`) to make the discard intent clear, but any non-`@local.*` name works.

## How definitions and references are matched

Helix matches references to definitions by comparing the text of the reference
node against the text of definitions visible from the current scope. Definitions
in inner scopes shadow those in outer scopes. If no definition is found the
reference is left with its `highlights.scm` highlight.

## Relationship with `highlights.scm`

The locals system runs alongside `highlights.scm`, not instead of it.
`highlights.scm` always determines the baseline highlight for a node.
`locals.scm` can override that highlight only when a `@local.reference`
successfully resolves to a `@local.definition`, and only for that specific
resolution. Non-`@local.*` captures in `locals.scm` (i.e. discards) have no
effect on `highlights.scm` results.
