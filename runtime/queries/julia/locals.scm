
(import_statement
 (identifier) @definition.import)
(variable_declaration
 (identifier) @local.definition)
(variable_declaration
 (tuple_expression
  (identifier) @local.definition))
(for_binding
 (identifier) @local.definition)
(for_binding
 (tuple_expression
  (identifier) @local.definition))

(assignment_expression
 (tuple_expression
  (identifier) @local.definition))
(assignment_expression
 (bare_tuple_expression
  (identifier) @local.definition))
(assignment_expression
 (identifier) @local.definition)

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
 name: (identifier) @definition.function) @local.scope
(macro_definition 
 name: (identifier) @definition.macro) @local.scope

(identifier) @local.reference

[
  (try_statement)
  (finally_clause)
  (quote_statement)
  (let_statement)
  (compound_expression)
  (for_statement)
] @local.scope
