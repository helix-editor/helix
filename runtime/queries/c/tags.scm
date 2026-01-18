(function_declarator
  declarator: [(identifier) (field_identifier)] @name) @definition.function

(preproc_function_def name: (identifier) @name) @definition.function

(preproc_def name: (identifier) @name) @definition.constant

(type_definition
  declarator: (type_identifier) @name) @definition.type

(struct_specifier
  name: (type_identifier) @name) @definition.struct

(enum_specifier
  name: (type_identifier) @name) @definition.type
