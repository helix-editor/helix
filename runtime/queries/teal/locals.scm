
(var_declaration
  declarators: (var_declarators
  (var (identifier)) @definition.var))

(var_assignment
  variables: (assignment_variables
    (var (identifier) @definition.var) @definition.associated))

(arg name: (identifier) @definition.parameter)

(anon_function) @scope
((function_statement
  (function_name) @definition.function) @scope
  (#set! definition.function.scope "parent"))

(program) @scope
(if_statement) @scope
(generic_for_statement (for_body) @scope)
(numeric_for_statement (for_body) @scope)
(repeat_statement) @scope
(while_statement (while_body) @scope)
(do_statement) @scope

(identifier) @reference

