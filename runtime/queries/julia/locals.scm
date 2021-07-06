
(import_statement
 (identifier) @definition.import)
(variable_declaration
 (identifier) @definition.var)
(variable_declaration
 (tuple_expression
  (identifier) @definition.var))
(for_binding
 (identifier) @definition.var)
(for_binding
 (tuple_expression
  (identifier) @definition.var))

(assignment_expression
 (tuple_expression
  (identifier) @definition.var))
(assignment_expression
 (bare_tuple_expression
  (identifier) @definition.var))
(assignment_expression
 (identifier) @definition.var)

(type_parameter_list
  (identifier) @definition.type)
(type_argument_list
  (identifier) @definition.type)
(struct_definition
  name: (identifier) @definition.type)

(parameter_list
 (identifier) @definition.parameter)
(typed_parameter
 (identifier) @definition.parameter
 (identifier))
(function_expression
 . (identifier) @definition.parameter)
(argument_list
 (typed_expression
  (identifier) @definition.parameter
  (identifier)))
(spread_parameter
 (identifier) @definition.parameter)

(function_definition
 name: (identifier) @definition.function) @scope
(macro_definition 
 name: (identifier) @definition.macro) @scope

(identifier) @reference

[
  (try_statement)
  (finally_clause)
  (quote_statement)
  (let_statement)
  (compound_expression)
  (for_statement)
] @scope
