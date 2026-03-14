(term_definition
  name: (regular_identifier) @name) @definition.function

(type_declaration
  (type_kw)
  (type_constructor
    ((type_name (regular_identifier) @name)) @definition.type))

(ability_declaration
  (ability_name) @type _) @definition.type
