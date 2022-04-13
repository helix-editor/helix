; https://github.com/connorlay/tree-sitter-eex/blob/f742f2fe327463335e8671a87c0b9b396905d1d1/queries/injections.scm

((directive (expression) @injection.content)
 (#set! injection.language "elixir"))

((partial_expression) @injection.content
 (#set! injection.language "elixir")
 (#set! injection.include-children)
 (#set! injection.combined))
