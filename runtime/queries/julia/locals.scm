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

(spread_parameter
  (identifier) @local.definition)

(function_expression
  . (identifier) @local.definition)
 
; ------
; Scopes
; ------

[
  (function_definition)
  (macro_definition)
] @local.scope

; ----------
; References
; ----------

(identifier) @local.reference
