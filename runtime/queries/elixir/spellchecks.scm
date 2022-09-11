; Elixir Code Comments
(comment) @spell

; Elixir Documentation
(unary_operator
  operator: "@"
  operand: (call
  target: ((identifier) @_identifier (#match? @_identifier "^(module|type|short)?doc$"))
    (arguments [
      (string (quoted_content) @spell)
      (sigil (quoted_content) @spell)
  ])))

; Phoenix Live View Component Macros
(call 
  (identifier) @_identifier
  (arguments
    (atom)+
    (keywords (pair 
      ((keyword) @_keyword (#eq? @_keyword "doc: "))
      [
        (string (quoted_content) @spell)
        (sigil (quoted_content) @spell)
      ]))
  (#match? @_identifier "^(attr|slot)$")))
