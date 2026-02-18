; inherits: c

(function_declarator
  declarator: (qualified_identifier name: (identifier) @definition.function))

(class_specifier
  name: (type_identifier) @definition.class
  body: (field_declaration_list))
