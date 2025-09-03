;; Scopes

[
  (module)
  (function_definition)
  (lambda)
] @local.scope

;; Definitions

; Parameters
(parameters
  (identifier) @local.definition.variable.parameter)
(parameters
  (typed_parameter
    (identifier) @local.definition.variable.parameter))
(parameters
  (default_parameter 
    name: (identifier) @local.definition.variable.parameter))
(parameters 
  (typed_default_parameter 
    name: (identifier) @local.definition.variable.parameter))
(parameters
  (list_splat_pattern ; *args
    (identifier) @local.definition.variable.parameter))
(parameters
  (dictionary_splat_pattern ; **kwargs
    (identifier) @local.definition.variable.parameter))
(parameters
  (identifier) @local.definition.variable.builtin
  (#any-of? @local.definition.variable.builtin "self" "cls")) ; label self/cls as builtin
    
(lambda_parameters
  (identifier) @local.definition.variable.parameter)
  
; Imports
(import_statement
  name: (dotted_name 
    (identifier) @local.definition.namespace))

(aliased_import
  alias: (identifier) @local.definition.namespace)

;; References

(identifier) @local.reference

