; -----------
; Definitions
; -----------

; Variables
(assignment
  (identifier) @local.definition)

(assignment
  (tuple_expression
    (identifier) @local.definition))

; Constants
(const_statement
  (assignment
    . (identifier) @local.definition))

; let/const bindings
(let_binding
  (identifier) @local.definition)

(let_binding
  (tuple_expression
    (identifier) @local.definition))

; For bindings
(for_binding
  (identifier) @local.definition)

(for_binding
  (tuple_expression
    (identifier) @local.definition))

; Types
(struct_definition
  name: (identifier) @local.definition)

(abstract_definition
  name: (identifier) @local.definition)

(abstract_definition
  name: (identifier) @local.definition)

(type_parameter_list
  (identifier) @local.definition)

; Module imports
(import_statement
  (identifier) @local.definition)

; Parameters
(parameter_list
  (identifier) @local.definition)

(optional_parameter
  .
  (identifier) @local.definition)

(slurp_parameter
  (identifier) @local.definition)

(typed_parameter
  parameter: (identifier) @local.definition
  (_))

; Single parameter arrow function
(function_expression
  .
  (identifier) @local.definition)

; Function/macro definitions
(function_definition
  name: (identifier) @local.definition) @local.scope

(short_function_definition
  name: (identifier) @local.definition) @local.scope

(macro_definition
  name: (identifier) @local.definition) @local.scope

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

