; Scopes

[
  (chunk)
  (function_declaration)
  (function_definition)
  (do_statement)
  (while_statement)
  (repeat_statement)
  (if_statement)
  (for_statement)
] @local.scope

; Definitions

(parameters
  (identifier) @local.definition.variable.parameter)

; `self` in a method is an implicit parameter.
(parameters
  (identifier) @local.definition.variable.builtin
  (#eq? @local.definition.variable.builtin "self"))

(for_numeric_clause
  name: (identifier) @local.definition.variable)

(for_generic_clause
  (variable_list
    (variable
      (identifier) @local.definition.variable)))

; `local x, y = ...`
(variable_declaration
  (assignment_statement
    (variable_list
      (variable
        (identifier) @local.definition.variable))))
(variable_declaration
  (variable_list
    (variable
      (identifier) @local.definition.variable)))

; References

(identifier) @local.reference

; Field/member names are not variable references.
(dot_index_expression
  field: (identifier) @_)
(method_index_expression
  method: (identifier) @_)
(field
  name: (identifier) @_)
