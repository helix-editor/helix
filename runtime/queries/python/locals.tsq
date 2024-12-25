;; Scopes

[
  (module)
  (function_definition)
  (lambda)
] @local.scope

;; Definitions

; Parameters
(parameters
  (identifier) @local.definition)
(parameters
  (typed_parameter
    (identifier) @local.definition))
(parameters
  (default_parameter 
    name: (identifier) @local.definition))
(parameters 
  (typed_default_parameter 
    name: (identifier) @local.definition))
(parameters
  (list_splat_pattern ; *args
    (identifier) @local.definition))
(parameters
  (dictionary_splat_pattern ; **kwargs
    (identifier) @local.definition))
    
(lambda_parameters
  (identifier) @local.definition)
  
; Imports
(import_statement
  name: (dotted_name 
    (identifier) @local.definition))

(aliased_import
  alias: (identifier) @local.definition)

;; References

(identifier) @local.reference

