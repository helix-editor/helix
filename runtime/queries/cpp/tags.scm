; inherits: c

(function_declarator
  declarator: (qualified_identifier name: (identifier) @definition.function))

(struct_specifier
  name: (type_identifier) @definition.struct
  body: (field_declaration_list))

(class_specifier
  name: (type_identifier) @definition.class
  body: (field_declaration_list))
