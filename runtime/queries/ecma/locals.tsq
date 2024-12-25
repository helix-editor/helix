; Scopes
;-------

[
  (statement_block)
  (function)
  (arrow_function)
  (function_declaration)
  (method_definition)
] @local.scope

; Definitions
;------------

; ...i
(rest_pattern
  (identifier) @local.definition)

; { i }
(object_pattern
  (shorthand_property_identifier_pattern) @local.definition)

; { a: i }
(object_pattern
  (pair_pattern
    value: (identifier) @local.definition))

; [ i ]
(array_pattern
  (identifier) @local.definition)

; i => ...
(arrow_function
  parameter: (identifier) @local.definition)

; const/let/var i = ...
(variable_declarator
  name: (identifier) @local.definition)

; References
;------------

(identifier) @local.reference
