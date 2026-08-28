; The function name lives in the signature's call_expression (optionally wrapped
; in a where_expression); short-form `f(x) = …` is an assignment with a call LHS.
(function_definition
  (signature
    [
      (call_expression . (identifier) @name)
      (where_expression (call_expression . (identifier) @name))
    ])) @definition.function

((assignment
  . (call_expression . (identifier) @name)) @definition.function)

(macro_definition
  (signature (call_expression . (identifier) @name))) @definition.macro

(module_definition
  name: (identifier) @name) @definition.module

; Type name(s) live in a type_head; unwrap the `<: Super` (binary_expression)
; and `{T}` (parametrized_type_expression) wrappers to the leading identifier.
(struct_definition
  (type_head [
    (identifier) @name
    (binary_expression . (identifier) @name)
    (parametrized_type_expression . (identifier) @name)
    (binary_expression . (parametrized_type_expression . (identifier) @name))
  ])) @definition.struct

(abstract_definition
  (type_head [
    (identifier) @name
    (binary_expression . (identifier) @name)
    (parametrized_type_expression . (identifier) @name)
    (binary_expression . (parametrized_type_expression . (identifier) @name))
  ])) @definition.type

(primitive_definition
  (type_head [
    (identifier) @name
    (binary_expression . (identifier) @name)
    (parametrized_type_expression . (identifier) @name)
  ])) @definition.type

(const_statement
  (assignment . (identifier) @name)) @definition.constant
