[
  (command)
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
] @indent

[
  "}"
  "]"
  ")"
  ";"
] @outdent

; Strings are opaque and space-sensitive, don't indent them
(string) @opaque
