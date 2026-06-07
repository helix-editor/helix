## Adding indent queries

Helix uses tree-sitter to correctly indent new lines. This requires a
tree-sitter grammar and an `indent.scm` query file placed in
`runtime/queries/{language}/indents.scm`.

The indent level of a line is **the number of `@indent` scopes that contain
it**. An `@indent` capture on a node opens a scope spanning the lines *after*
the node's first line through its last line; a line is indented once for every
such scope wrapping it. (A few capture types adjust this — `@outdent` cancels a
level, `@align` aligns to a column, etc. (see [Capture types](#capture-types).)

Note that it matters where these scopes *begin*: multiple scopes that open on
the same physical line only increase the indent by 1 (the same-line rule). See
[Capture types](#capture-types).

By default, Helix uses the `hybrid` indentation heuristic. This means that
indent queries are not used to compute the expected absolute indentation of a
line but rather the expected difference in indentation between the new and an
already existing line. This difference is then added to the actual indentation
of the already existing line. Since this makes errors in the indent queries
harder to find, it is recommended to disable it when testing via
`:set indent-heuristic tree-sitter`. The rest of this guide assumes that
the `tree-sitter` heuristic is used.

## Indent queries

When Helix is inserting a new line through `o`, `O`, or `<ret>`, to determine
the indent level for the new line, the query in `indents.scm` is run on the
document. The starting position of the query is the end of the line above where
a new line will be inserted.

For `o`, the inserted line is the line below the cursor, so that starting
position of the query is the end of the current line.

```rust
fn need_hero(some_hero: Hero, life: Life) -> {
    matches!(some_hero, Hero { // ←─────────────────╮
        strong: true,//←╮  ↑  ↑                     │
        fast: true,  // │  │  ╰── query start       │
        sure: true,  // │  ╰───── cursor            ├─ traversal 
        soon: true,  // ╰──────── new line inserted │  start node
    }) &&            //                             │
//  ↑                                               │
//  ╰───────────────────────────────────────────────╯
    some_hero > life
}
```

For `O`, the newly inserted line is the *current* line, so the starting position
of the query is the end of the line above the cursor.

```rust
fn need_hero(some_hero: Hero, life: Life) -> { // ←─╮
    matches!(some_hero, Hero { // ←╮          ↑     │
        strong: true,//    ↑   ╭───╯          │     │
        fast: true,  //    │   │ query start ─╯     │
        sure: true,  //    ╰───┼ cursor             ├─ traversal
        soon: true,  //        ╰ new line inserted  │  start node
    }) &&            //                             │
    some_hero > life //                             │
} // ←──────────────────────────────────────────────╯
```

From this starting node, the syntax tree is walked up to the root, collecting
every `@indent`/`@outdent`/`@align`/... capture on an ancestor. Each `@indent`
ancestor whose scope *contains* the line counts for one indent level (scopes
opening on the same line collapse to one); the [capture types](#capture-types)
describe the adjustments.

### Capture types

- `@indent`:
  Open an indent scope on this node — every line it contains is indented one
  more level. Scopes that open on the same line collapse: multiple `@indent`
  beginning on one line only add 1. A node captured by *both* `@indent` and
  `@outdent` contributes nothing (its level is cancelled) — used e.g. for a
  nested `else if` that should not stack a second level.
  By default the scope opens at the node's own first line; see the
  [`header`](#scope-header) scope to open it at the parent (header) line instead.
- `@outdent`:
  Decrease by 1 the indent of the line on which this (usually a closing token
  like `}`/`)`/`]`, or a keyword like `else`) begins.
- `@indent.always`:
  Like `@indent` but does *not* collapse — multiple on the same line each add a
  level. The net level contribution is `@indent.always` − `@outdent.always`.
- `@outdent.always`:
  Like `@outdent` but stacks, the counterpart to `@indent.always`.
- `@align` (default scope `all`):
  Align everything inside this node to some anchor. The anchor is given
  by the start of the node captured by `@anchor` in the same pattern.
  Every pattern with an `@align` should contain exactly one `@anchor`.
  Indent (and outdent) for nodes below (in terms of their starting line)
  the `@align` node is added to the indentation required for alignment.
- `@extend`:
  Extend the range of this node to the end of the line and to lines that are
  indented more than the line that this node starts on. This is useful for
  languages like Python, where for the purpose of indentation some nodes (like
  functions or classes) should also contain indented lines that follow them.
- `@extend.prevent-once`:
  Prevents the first extension of an ancestor of this node. For example, in Python
  a return expression always ends the block that it is in. Note that this only
  stops the extension of the next `@extend` capture. If multiple ancestors are
  captured, only the extension of the innermost one is prevented. All other
  ancestors are unaffected (regardless of whether the innermost ancestor would
  actually have been extended).
- `@opaque`:
  Mark a literal body such as a string, heredoc, or block comment. Lines that
  begin inside the captured node keep their existing indentation instead of being
  reindented, so the contents of multi-line literals are left untouched.

#### `@indent` / `@outdent`

Consider this example:

```rust
fn shout(things: Vec<Thing>) {
    //                       ↑
    //                       ├───────────────────────╮ indent level
    //                    @indent                    ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄
    //                                               │
    let it_all = |out| { things.filter(|thing| { //  │      1
    //                 ↑                       ↑     │
    //                 ├───────────────────────┼─────┼┄┄┄┄┄┄┄┄┄┄┄┄┄┄
    //              @indent                 @indent  │
    //                                               │      2
        thing.can_do_with(out) //                    │
    })}; //                                          ├┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  //↑↑↑                                              │      1
} //╰┼┴──────────────────────────────────────────────┴┄┄┄┄┄┄┄┄┄┄┄┄┄┄
// 3x @outdent
```

```scm
((block) @indent)
["}" ")"] @outdent
```

Note how on the second line two blocks open on the same line: since both scopes
*begin* on that line they collapse, for a net increase of 1. On the last line,
the three block scopes all contain it, but it begins with three `@outdent` `}`
tokens that cancel them, so the line lands back at the enclosing level.

#### Same-line collapse

The collapse above is a deliberate, load-bearing invariant:

> A line is indented by **one level per physical line on which a containing
> `@indent` scope opens**, not one level per scope. Several `@indent` scopes
> that *open* on the same line together add a single level.

This is what makes **method/builder chains flatten instead of staircasing**.
Grammars typically nest a chain so each `.method()` link is a `call` inside a
`member_expression` inside the previous link, and several of those nodes *begin*
on the receiver's line:

```rust
let x = thing       // ← chain opens here
    .foo()          // each link aligned one level in,
    .bar()          // not progressively deeper
    .baz();
```

```scm
(call_expression) @indent
```

Even though many `call_expression`/`member_expression` scopes contain the
`.bar()` line, they all *open* on the `thing` line, so they collapse to a single
level and the continuation lines line up. Without the collapse, every link would
add a level and the chain would stair-step to the right.

If you instead want each scope to count even when several open on one line (for
example YAML's "list item *and* map both start on the same line") opt out with
`@indent.always` (described below), which does not collapse.

#### `@extend` / `@extend.prevent-once`

For an example of where `@extend` can be useful, consider Python, which is
whitespace-sensitive.

```scm
]
  (parenthesized_expression)
  (function_definition)
  (class_definition)
] @indent

```

```python
class Hero:
    def __init__(self, strong, fast, sure, soon):#  ←─╮
        self.is_strong = strong #                     │
        self.is_fast = fast     # ╭─── query start    │
        self.is_sure = sure     # │ ╭─ cursor         │
        self.is_soon = soon     # │ │                 │
        #     ↑            ↑      │ │                 │
        #     │            ╰──────╯ │                 │
        #     ╰─────────────────────╯                 │
        #                                             ├─ traversal
    def need_hero(self, life):         #              │  start node
        return (                       #              │
            self.is_strong             #              │
            and self.is_fast           #              │
            and self.is_sure           #              │
            and self.is_soon           #              │
            and self > life            #              │
        ) # ←─────────────────────────────────────────╯
```

Without braces to catch the scope of the function, the smallest descendant of
the cursor on a line feed ends up being the entire inside of the class. Because
of this, it will miss the entire function node and its indent capture, leading
to an indent level one too small.

To address this case, `@extend` tells helix to "extend" the captured node's span
to the line feed and every consecutive line that has a greater indent level than
the line of the node.

```scm
(parenthesized_expression) @indent

]
  (function_definition)
  (class_definition)
] @indent @extend

```

```python
class Hero:
    def __init__(self, strong, fast, sure, soon):#  ←─╮
        self.is_strong = strong #                     │
        self.is_fast = fast     # ╭─── query start    ├─ traversal
        self.is_sure = sure     # │ ╭─ cursor         │  start node
        self.is_soon = soon     # │ │ ←───────────────╯
        #     ↑            ↑      │ │                 
        #     │            ╰──────╯ │
        #     ╰─────────────────────╯
    def need_hero(self, life):
        return (
            self.is_strong
            and self.is_fast
            and self.is_sure
            and self.is_soon
            and self > life
        )
```

Furthermore, there are some cases where extending to everything with a greater
indent level may not be desirable. Consider the `need_hero` function above. If
our cursor is on the last line of the returned expression.

```python
class Hero:
    def __init__(self, strong, fast, sure, soon):
        self.is_strong = strong
        self.is_fast = fast
        self.is_sure = sure
        self.is_soon = soon

    def need_hero(self, life):
        return (
            self.is_strong
            and self.is_fast
            and self.is_sure
            and self.is_soon
            and self > life
        ) # ←─── cursor
    #←────────── where cursor should go on new line
```

In Python, the are a few tokens that will always end a scope, such as a return
statement. Since the scope ends, so should the indent level. But because the
function span is extended to every line with a greater indent level, a new line
would just continue on the same level. And an `@outdent` would not help us here
either, since it would cause everything in the parentheses to become outdented
as well.

To help, we need to signal an end to the extension. We can do this with
`@extend.prevent-once`.

```scm
(parenthesized_expression) @indent

]
  (function_definition)
  (class_definition)
] @indent @extend

(return_statement) @extend.prevent-once
```

#### Brace-less bodies

A brace-less single-statement body — `if (cond)` with its statement on the next
line and no `{}` — is a body the indent must wrap, but the body node's *own*
first line is the line that needs the indent (there is no separate opening line).
Capture the body and give it the [`header`](#scope-header) scope, which opens the
scope at the **header** (the captured node's parent) line instead of the body's
own line, so the body line is contained:

```scm
(if_statement
  consequence: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "header"))
(while_statement
  body: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "header"))
```

The query already names exactly the brace-less body (the `consequence:` /
`body:` field, with `#not-kind-eq?` skipping the braced form the surrounding
block indents), so the engine just opens that capture's scope at its parent — it
does not itself inspect node kinds or field names. This works for a multi-line
brace-less body too (e.g. one returning a lambda). Give `else` and `do / while`
their own pattern (on the `alternative` / `body` field) so the trailing `else` /
`while` keyword line is not indented along with the body.

#### `@indent.always` / `@outdent.always`

As mentioned before, normally if there is more than one `@indent` or `@outdent`
capture on the same line, they are combined.

Sometimes, there are cases when you may want to ensure that every indent capture
is additive, regardless of how many occur on the same line. Consider this
example in YAML.

```yaml
  - foo: bar
# ↑ ↑
# │ ╰─────────────── start of map
# ╰───────────────── start of list element
    baz: quux # ←─── cursor
    # ←───────────── where the cursor should go on a new line
    garply: waldo
  - quux:
      bar: baz
    xyzzy: thud
    fred: plugh
```

In YAML, you often have lists of maps. In these cases, the syntax is such that
the list element and the map both start on the same line. But we really do want
to start an indentation for each of these so that subsequent keys in the map
hang over the list and align properly. This is where `@indent.always` helps.

```scm
((block_sequence_item) @item @indent.always @extend
  (#not-one-line? @item))

((block_mapping_pair
    key: (_) @key
    value: (_) @val
    (#not-same-line? @key @val)
  ) @indent.always @extend
)
```

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

- `#any-of?`/`#not-any-of?`:
The first argument (a capture) must/must not be one of the other arguments
(strings).

Additionally, we support some custom predicates for indent queries:

- `#not-kind-eq?`:
The kind of the first argument (a capture) must not be equal to the second
argument (a string).

- `#same-line?`/`#not-same-line?`:
The captures given by the 2 arguments must/must not start on the same line.

- `#one-line?`/`#not-one-line?`:
The captures given by the fist argument must/must span a total of one line.

### <a name="scope-header"></a>The `header` scope

By default an `@indent` scope opens at the captured node's own first line, so the
lines *inside* it are indented and its first line is not. Sometimes the node you
must capture *is* the line that needs indenting — a brace-less body such as the
statement after `if (cond)` (see [Brace-less bodies](#brace-less-bodies)). For
this, set `scope` to `header`:

```scm
(if_statement
  consequence: (_) @indent
  (#not-kind-eq? @indent "compound_statement")
  (#set! "scope" "header"))
```

`header` opens the scope at the captured node's **parent** (the `if_statement`
header) line instead of the node's own line, so the body's first line is
contained and indented. The query selects exactly the body, so the engine just
honours the annotation — it does not match on node kinds or field names itself.

(Older queries used `tail` / `all` scopes to control whether a node's first line
was included; under the containment model those are no longer needed and have
been removed. `header` is the one scope the engine reads.)

## Testing

`cargo xtask indent-check [language]` checks the queries against the fixtures in
`tests/indent/<language-id>.<ext>` in both modes: re-indenting each line
and simulating a newline typed after it, so a rule that is correct one way but
wrong the other is caught.
