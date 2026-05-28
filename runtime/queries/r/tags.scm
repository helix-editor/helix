(binary_operator
  lhs: [(identifier) (string)] @name
  operator: "<-"
  rhs: (function_definition)) @definition.function

(binary_operator
  lhs: [(identifier) (string)] @name
  operator: "="
  rhs: (function_definition)) @definition.function
