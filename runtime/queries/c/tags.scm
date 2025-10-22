(function_declarator
  declarator: [(identifier) (field_identifier)] @definition.function)

(preproc_function_def name: (identifier) @definition.function)

(type_definition
  declarator: (type_identifier) @definition.type)

(preproc_def name: (identifier) @definition.constant)
