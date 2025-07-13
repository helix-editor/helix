; queries for helix to do automatic indentation upon hitting enter
; TODO: needs more work, cover more cases
[
  (entity)
  (method)
  (behavior)
  (constructor)
  (block)
  (tuple)
  (grouped)
] @indent
(match_case body: (block) @indent)
; ffi_call and call
(_ arguments: (_) @indent)
(assignment right: (_) @indent
 (#set! "scope" "all")
)

[
  (params)
  (object)
  ("if")
] @extend
(lambda params: (_) @extend)

[
  "end"
  "}"
  "]"
  ")"
  "|"
] @outdent
