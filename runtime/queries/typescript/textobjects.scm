; inherits: ecma

[
  (interface_declaration 
    body:(_) @class.inside)
  (type_alias_declaration 
    value: (_) @class.inside)
] @class.around
