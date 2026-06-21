; Function definitions
(function_declaration
  name: [
    (identifier) @name
    (dot_index_expression field: (identifier) @name)
    (method_index_expression method: (identifier) @name)
  ]) @definition.function

; `x = function() ... end`, `M.x = function() ... end`
(assignment_statement
  (variable_list
    name: [
      (identifier) @name
      (dot_index_expression field: (identifier) @name)
    ])
  (expression_list
    value: (function_definition))) @definition.function
