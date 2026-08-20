(object_definition
  name: (type_identifier) @name) @definition.type

(partial_definition
  name: (type_identifier) @name) @definition.type

(caveat_definition
  name: (func_identifier) @name) @definition.function

(relation
  name: (field_identifier) @name) @definition.field

(permission
  name: (method_identifier) @name) @definition.method

(partial_reference
  name: (identifier) @name) @reference.type

(relation_type
  (type_identifier) @name) @reference.type

(wildcard_type
  (type_identifier) @name) @reference.type

(reference_type
  (type_identifier) @name) @reference.type

(reference_type
  (field_identifier) @name) @reference.call

(caveat_with_expiration
  (func_identifier) @name) @reference.call

(userset
  (field_identifier) @name) @reference.call

(arrow_expression
  (field_identifier) @name) @reference.call
