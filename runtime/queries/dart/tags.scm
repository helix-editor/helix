(class_definition
  name: (identifier) @name) @definition.class
(enum_declaration
  name: (identifier) @name) @definition.enum
(mixin_declaration
  (identifier) @name) @definition.interface
(extension_declaration
  name: (identifier) @name) @definition.class
(function_signature
  name: (identifier) @name) @definition.function
(constructor_signature
  name: (identifier) @name) @definition.function
(type_alias
  "typedef" . (type_identifier) @name) @definition.type
