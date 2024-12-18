
(var_declaration
  declarators: (var_declarators
  (var (identifier)) @local.definition))

(var_assignment
  variables: (assignment_variables
    (var (identifier) @local.definition)))

(arg name: (identifier) @local.definition)

(anon_function) @local.scope
((function_statement
  (function_name) @local.definition) @local.scope)

(program) @local.scope
(if_statement) @local.scope
(generic_for_statement (for_body) @local.scope)
(numeric_for_statement (for_body) @local.scope)
(repeat_statement) @local.scope
(while_statement (while_body) @local.scope)
(do_statement) @local.scope

(identifier) @local.reference

