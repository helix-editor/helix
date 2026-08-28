(class_declaration "class" name: (type_identifier) @name) @definition.class
(class_declaration "struct" name: (type_identifier) @name) @definition.struct
(class_declaration "enum" name: (type_identifier) @name) @definition.enum
(class_declaration "extension" name: (user_type (type_identifier) @name)) @definition.class
(protocol_declaration name: (type_identifier) @name) @definition.interface
(typealias_declaration name: (type_identifier) @name) @definition.type
(function_declaration name: (simple_identifier) @name) @definition.function
(protocol_function_declaration name: (simple_identifier) @name) @definition.function
(property_declaration
  name: (pattern bound_identifier: (simple_identifier) @name)) @definition.constant
