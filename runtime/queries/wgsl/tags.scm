(struct_declaration
  name: (identifier) @definition.struct)

(function_declaration
  name: (identifier) @definition.function)

(global_variable_declaration
  (variable_declaration
    (variable_identifier_declaration
      name: (identifier) @definition.constant)))
