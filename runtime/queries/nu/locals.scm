; Scopes
(function_definition) @scope

; Definitions
(variable_declaration 
  name: (identifier) @definition.var)

(function_definition
  func_name: (identifier) @definition.function)

; References
(value_path) @reference
(word) @reference
