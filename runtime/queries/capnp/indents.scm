[
  (annotation_targets)
  (const_list)
  (enum)
  (interface)
  (implicit_generics)
  (generics)
  (group)
  (method_parameters)
  (named_return_types)
  (struct)
  (struct_shorthand)
  (union)
] @indent

((struct_shorthand (property)) @aligned_indent
  (#set! "delimiter" "()"))

((const_list (const_value)) @aligned_indent
  (#set! "delimiter" "[]"))

[
  "}"
  ")"
] @indent_end

[ "{" "}" ] @branch

[ "(" ")" ] @branch

[
  (ERROR)
  (comment)
] @auto
