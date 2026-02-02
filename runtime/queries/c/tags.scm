(function_declarator
  declarator: [(identifier) (field_identifier)] @definition.function)

(preproc_function_def name: (identifier) @definition.function)

(preproc_def name: (identifier) @definition.constant)

(type_definition
  declarator: (type_identifier) @definition.type)

(struct_specifier
  name: (type_identifier) @definition.struct)

(enum_specifier
  name: (type_identifier) @definition.type)
