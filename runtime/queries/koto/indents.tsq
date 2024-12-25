[
  (list)
  (map)
  (tuple)
] @indent

[
  (for)
  (else_if)
  (else)
  (match)
  (switch)
  (until)
  (while)
] @indent @extend

(assign
  "=" @indent @extend
  !rhs
)
(assign
  "=" @indent @extend
  rhs: (_) @anchor
  (#not-same-line? @indent @anchor)
)

(if
  condition: (_) @indent @extend
  !then
)
(if
  condition: (_) @indent @extend
  then: (_) @anchor
  (#not-same-line? @indent @anchor)
)

(function
  (args) @indent @extend
  !body
)
(function
  (args) @indent @extend
  body: (_) @anchor
  (#not-same-line? @indent @anchor)
)

(match_arm
  "then" @indent @extend
  !then
)
(match_arm
  "then" @indent @extend
  then: (_) @anchor
  (#not-same-line? @indent @anchor)
)

[
  "}"
  "]"
  ")"
] @outdent
