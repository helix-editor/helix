[
  (block)
  (comment_block)
  (test_list)
  ; string list
  (
    "["
    (string)
    ("," (string))*
    "]"
  )
] @rainbow.scope

[
  "[" "]"
  "(" ")"
  "{" "}"
] @rainbow.bracket
