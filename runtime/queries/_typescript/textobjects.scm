[
  (interface_declaration 
    body:(_) @class.inside)
  (type_alias_declaration 
    value: (_) @class.inside)
] @class.around

(enum_body
  (_) @entry.around)

(enum_assignment (_) @entry.inside)

