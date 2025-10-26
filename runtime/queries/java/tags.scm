(class_declaration
  name: (identifier) @definition.class)

(interface_declaration
  name: (identifier) @definition.interface)

(record_declaration
  name: (identifier) @definition.class)

(enum_declaration
  name: (identifier) @defintion.class)

(method_declaration
  name: (identifier) @definition.function)

(constructor_declaration
  name: (identifier) @definition.function)

(compact_constructor_declaration
  name: (identifier) @definition.function)

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @definition.constant))

(enum_constant
  name: (identifier) @definition.constant)

; References

(superclass
  (type_identifier) @reference.class)

(super_interfaces
  (type_list
    (type_identifier) @reference.interface))

(object_creation_expression
  type: (_) @reference.type)

(type_identifier) @reference.type

(method_invocation
  name: (identifier) @reference.function)

(field_access
  field: (identifier) @reference.constant)
