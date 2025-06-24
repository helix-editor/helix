(identifier) @local.reference
(assignment_expression
   left: (identifier) @local.definition.variable)
(macro_statement
  (parameter_list
    (identifier) @local.definition.variable.parameter))
(macro_statement) @local.scope
