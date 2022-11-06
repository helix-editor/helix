; -----------
; Definitions
; -----------

; Imports
(import_statement
  (identifier) @local.definition)
  
; Constants
(const_statement
  (variable_declaration
    . (identifier) @local.definition))

; Parameters
(parameter_list
  (identifier) @local.definition)

(typed_parameter
  . (identifier) @local.definition)

(optional_parameter .
  (identifier) @local.definition)

(slurp_parameter
  (identifier) @local.definition)

(function_expression
  . (identifier) @local.definition)
 
; ------
; Scopes
; ------

[
  (function_definition)
  (short_function_definition)
  (macro_definition)
] @local.scope

; ----------
; References
; ----------

(identifier) @local.reference
