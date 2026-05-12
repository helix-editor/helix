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
