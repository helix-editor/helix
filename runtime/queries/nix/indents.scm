[
  ; Bracket like
  (let_expression)
  (attrset_expression)
  (rec_attrset_expression)
  (let_attrset_expression)
  (parenthesized_expression)
  (list_expression)
  (indented_string_expression)

  ; Binding
  (binding)
  (inherit)
  (inherit_from)
  (formals)
  (with_expression)

  ; Conditional
  (if_expression)
] @indent

(inherit_from expression: (_) @indent)

; special case where formals are on same line as the next block
((_ (_ ((formals) . [
  (let_expression)
  (attrset_expression)
  (rec_attrset_expression)
  (let_attrset_expression)
  (parenthesized_expression)
  (list_expression)
  (indented_string_expression)
] @outdent))) @_code
  (#not-kind-eq? @_code "source_code")
  (#not-kind-eq? @_code "function_expression")
  (#not-kind-eq? @_code "if_expression")
  (#not-kind-eq? @_code "with_expression")
  (#not-kind-eq? @_code "inherit"))

; avoid extra indent in let and if expressions
(let_expression body: [
  (let_expression)
  (attrset_expression)
  (rec_attrset_expression)
  (let_attrset_expression)
  (parenthesized_expression)
  (list_expression)
  (indented_string_expression)
  (binding)
  (inherit)
  (inherit_from)
  (formals)
] @outdent)

(if_expression [
  (let_expression)
  (attrset_expression)
  (rec_attrset_expression)
  (let_attrset_expression)
  (list_expression)
  (indented_string_expression)
  (formals)
] @outdent)

; functions only indent body in parens and in blocks
(parenthesized_expression
 expression: (function_expression) @indent)

(let_expression body:
  (function_expression) @indent)