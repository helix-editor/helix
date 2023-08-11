# Adding indent queries

Helix uses tree-sitter to correctly indent new lines. This requires a tree-
sitter grammar and an `indent.scm` query file placed in `runtime/queries/
{language}/indents.scm`. The indentation for a line is calculated by traversing
the syntax tree from the lowest node at the beginning of the new line (see
[Indent queries](#indent-queries)). Each of these nodes contributes to the total
indent when it is captured by the query (in what way depends on the name of
the capture.

Note that it matters where these added indents begin. For example,
multiple indent level increases that start on the same line only increase
the total indent level by 1. See [Capture types](#capture-types).

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

From this starting node, the syntax tree is traversed up until the root node.
Each indent capture is collected along the way, and then combined according to
their [capture types](#capture-types) and [scopes](#scopes) to a final indent
level for the line.

### Capture types

- `@indent` (default scope `tail`):
  Increase the indent level by 1. Multiple occurrences in the same line *do not*
  stack. If there is at least one `@indent` and one `@outdent` capture on the
  same line, the indent level isn't changed at all.
- `@outdent` (default scope `all`):
  Decrease the indent level by 1. The same rules as for `@indent` apply.
- `@indent.always` (default scope `tail`):
  Increase the indent level by 1. Multiple occurrences on the same line *do*
  stack. The final indent level is `@indent.always` – `@outdent.always`. If
  an `@indent` and an `@indent.always` are on the same line, the `@indent` is
  ignored.
- `@outdent.always` (default scope `all`):
  Decrease the indent level by 1. The same rules as for `@indent.always` apply.
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

Note how on the second line, we have two blocks begin on the same line. In this
case, since both captures occur on the same line, they are combined and only
result in a net increase of 1. Also note that the closing `}`s are part of the
`@indent` captures, but the 3 `@outdent`s also combine into 1 and result in that
line losing one indent level.

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

Additionally, we support some custom predicates for indent queries:

- `#not-kind-eq?`:
The kind of the first argument (a capture) must not be equal to the second
argument (a string).

- `#same-line?`/`#not-same-line?`:
The captures given by the 2 arguments must/must not start on the same line.

- `#one-line?`/`#not-one-line?`:
The captures given by the fist argument must/must span a total of one line.

### Scopes

Added indents don't always apply to the whole node. For example, in most
cases when a node should be indented, we actually only want everything
except for its first line to be indented. For this, there are several
scopes (more scopes may be added in the future if required):

- `tail`:
This scope applies to everything except for the first line of the
captured node.
- `all`:
This scope applies to the whole captured node. This is only different from
`tail` when the captured node is the first node on its line.

For example, imagine we have the following function

```rust
fn aha() { // ←─────────────────────────────────────╮
  let take = "on me";  // ←──────────────╮  scope:  │
  let take = "me on";             //     ├─ "tail"  ├─ (block) @indent
  let ill = be_gone_days(1 || 2); //     │          │
} // ←───────────────────────────────────┴──────────┴─ "}" @outdent
                                         //                scope: "all"
```

We can write the following query with the `#set!` declaration:

  ```scm
  ((block) @indent
   (#set! "scope" "tail"))
  ("}" @outdent
   (#set! "scope" "all"))
  ```

As we can see, the "tail" scope covers the node, except for the first line.
Everything up to and including the closing brace gets an indent level of 1.
Then, on the closing brace, we encounter an outdent with a scope of "all", which
means the first line is included, and the indent level is cancelled out on this
line. (Note these scopes are the defaults for `@indent` and `@outdent`—they are
written explicitly for demonstration.)