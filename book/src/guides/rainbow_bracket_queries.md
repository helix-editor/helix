# Adding Rainbow Bracket Queries

Helix uses `rainbows.scm` tree-sitter query files to provide rainbow bracket
functionality.

Tree-sitter queries are documented in the tree-sitter online documentation.
If you're writing queries for the first time, be sure to check out the section
on [syntax highlighting queries] and on [query syntax].

Rainbow queries have two captures: `@rainbow.scope` and `@rainbow.bracket`.
`@rainbow.scope` should capture any node that increases the nesting level
while `@rainbow.bracket` should capture any bracket nodes. Put another way:
`@rainbow.scope` switches to the next rainbow color for all nodes in the tree
under it while `@rainbow.bracket` paints captured nodes with the current
rainbow color.

For an example, let's add rainbow queries for the tree-sitter query (TSQ)
language itself. These queries will go into a
`runtime/queries/tsq/rainbows.scm` file in the repository root.

First we'll add the `@rainbow.bracket` captures. TSQ only has parentheses and
square brackets:

```tsq
["(" ")" "[" "]"] @rainbow.bracket
```

The ordering of the nodes within the alternation (square brackets) is not
taken into consideration.

> Note: Why are these nodes quoted? Most syntax highlights capture text
> surrounded by parentheses. These are _named nodes_ and correspond to the
> names of rules in the grammar. Brackets are usually written in tree-sitter
> grammars as literal strings, for example:
>
> ```js
> {
>   // ...
>   arguments: seq("(", repeat($.argument), ")"),
>   // ...
> }
> ```
>
> Nodes written as literal strings in tree-sitter grammars may be captured
> in queries with those same literal strings.

Then we'll add `@rainbow.scope` captures. The easiest way to do this is to
view the `grammar.js` file in the tree-sitter grammar's repository. For TSQ,
that file is [here][tsq grammar.js]. As we scroll down the `grammar.js`, we
see that the `(alternation)`, (L36) `(group)` (L57), `(named_node)` (L59),
`(predicate)` (L87) and  `(wildcard_node)` (L97) nodes all contain literal
parentheses or square brackets in their definitions. These nodes are all
direct parents of brackets and happen to also be the nodes we want to change
to the next rainbow color, so we capture them as `@rainbow.scope`.

```tsq
[
  (group)
  (named_node)
  (wildcard_node)
  (predicate)
  (alternation)
] @rainbow.scope
```

This strategy works as a rule of thumb for most programming and configuration
languages. Markup languages can be trickier and may take additional
experimentation to find the correct nodes to use for scopes and brackets.

The `:tree-sitter-subtree` command shows the syntax tree under the primary
selection in S-expression format and can be a useful tool for determining how
to write a query.

### Properties

The `rainbow.include-children` property may be applied to `@rainbow.scope`
captures. By default, all `@rainbow.bracket` captures must be direct descendant
of a node captured with `@rainbow.scope` in a syntax tree in order to be
highlighted. The `rainbow.include-children` property disables that check and
allows `@rainbow.bracket` captures to be highlighted if they are direct or
indirect descendants of some node captured with `@rainbow.scope`.

For example, this property is used in the HTML rainbow queries.

For a document like `<a>link</a>`, the syntax tree is:

```tsq
(element                   ; <a>link</a>
  (start_tag               ; <a>
    (tag_name))            ; a
  (text)                   ; link
  (end_tag                 ; </a>
    (tag_name)))           ; a
```

If we want to highlight the `<`, `>` and `</` nodes with rainbow colors, we
capture them as `@rainbow.bracket`:

```tsq
["<" ">" "</"] @rainbow.bracket
```

And we capture `(element)` as `@rainbow.scope` because `(element)` nodes nest
within each other: they increment the nesting level and switch to the next
color in the rainbow.

```tsq
(element) @rainbow.scope
```

But this combination of `@rainbow.scope` and `@rainbow.bracket` will not
highlight any nodes. `<`, `>` and `</` are children of the `(start_tag)` and
`(end_tag)` nodes. We can't capture `(start_tag)` and `(end_tag)` as
`@rainbow.scope` because they don't nest other elements. We can fix this case
by removing the requirement that `<`, `>` and `</` are direct descendants of
`(element)` using the `rainbow.include-children` property.

```tsq
((element) @rainbow.scope
 (#set! rainbow.include-children))
```

With this property set, `<`, `>`, and `</` will highlight with rainbow colors
even though they aren't direct descendents of the `(element)` node.

`rainbow.include-children` is not necessary for the vast majority of programming
languages. It is only necessary when the node that increments the nesting level
(changes rainbow color) is not the direct parent of the bracket node.

[syntax highlighting queries]: https://tree-sitter.github.io/tree-sitter/syntax-highlighting#highlights
[query syntax]: https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
[tsq grammar.js]: https://github.com/the-mikedavis/tree-sitter-tsq/blob/48b5e9f82ae0a4727201626f33a17f69f8e0ff86/grammar.js
