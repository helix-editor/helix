(class_declaration
  (type_identifier) @definition.class)

(object_declaration
  "object" (type_identifier) @definition.class)

(function_declaration
  (simple_identifier) @definition.function)

(property_declaration
  (variable_declaration
    (simple_identifier) @definition.constant))
