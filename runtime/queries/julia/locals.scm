; -----------
; Definitions
; -----------

; Constants
(const_statement
  (assignment
    . (identifier) @local.definition.constant))

; Parameters
(parameter_list
  (identifier) @local.definition.variable.parameter)

(optional_parameter
  .
  (identifier) @local.definition.variable.parameter)

(slurp_parameter
  (identifier) @local.definition.variable.parameter)

(typed_parameter
  parameter: (identifier) @local.definition.variable.parameter
  (_))

; Single parameter arrow function
(function_expression
  .
  (identifier) @local.definition.variable.parameter)

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
  (function_definition)
  (short_function_definition)
  (macro_definition)
] @local.scope 

