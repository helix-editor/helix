(unary_operator
  operator: "@"
  operand: (call
    target: (identifier) @spell.__attribute__
    (arguments
      [
        (string) @spell
        (charlist) @spell
        (sigil (quoted_content) @spell)
      ]))
  (#match? @spell.__attribute__ "^(moduledoc|typedoc|doc)$"))

(comment) @spell
