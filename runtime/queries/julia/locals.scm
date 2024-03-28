; -----------
; Definitions
; -----------

; Variables
(assignment
  (identifier) @local.definition.var)

(assignment
  (tuple_expression
    (identifier) @local.definition.var))

; Constants
(const_statement
  (assignment
    . (identifier) @local.definition))

; let/const bindings
(let_binding
  (identifier) @local.definition.var)

(let_binding
  (tuple_expression
    (identifier) @local.definition.var))

; For bindings
(for_binding
  (identifier) @local.definition.var)

(for_binding
  (tuple_expression
    (identifier) @local.definition.var))

; Types
(struct_definition
  name: (identifier) @local.definition.type)

(abstract_definition
  name: (identifier) @local.definition.type)

(abstract_definition
  name: (identifier) @local.definition.type)

(type_parameter_list
  (identifier) @local.definition.type)

; Module imports
(import_statement
  (identifier) @local.definition.import)

; Parameters
(parameter_list
  (identifier) @local.definition.parameter)

(optional_parameter
  .
  (identifier) @local.definition.parameter)

(slurp_parameter
  (identifier) @local.definition.parameter)

(typed_parameter
  parameter: (identifier) @local.definition.parameter
  (_))

; Single parameter arrow function
(function_expression
  .
  (identifier) @local.definition.parameter)

; Function/macro definitions
(function_definition
  name: (identifier) @local.definition.function) @local.scope

(short_function_definition
  name: (identifier) @local.definition.function) @local.scope

(macro_definition
  name: (identifier) @local.definition.macro) @local.scope

; ----------
; References
; ----------

(identifier) @local.reference
 
; ------
; Scopes
; ------

[
  (for_statement)
  (while_statement)
  (try_statement)
  (catch_clause)
  (finally_clause)
  (let_statement)
  (quote_statement)
  (do_clause)
] @local.scope 

