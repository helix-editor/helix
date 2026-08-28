; inherits: c

(function_declarator
  declarator: (qualified_identifier name: (identifier) @definition.function))

(class_specifier
  name: (type_identifier) @definition.class
  body: (field_declaration_list))

(namespace_definition
  name: (namespace_identifier) @definition.module)

(concept_definition
  name: (identifier) @definition.interface)

(alias_declaration
  name: (type_identifier) @definition.type)
