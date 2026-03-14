((method) @local.scope
 (#set! local.scope-inherits false))

[
  (lambda)
  (block)
  (do_block)
] @local.scope

(block_parameter (identifier) @local.definition.variable.parameter)
(block_parameters (identifier) @local.definition.variable.parameter)
(destructured_parameter (identifier) @local.definition.variable.parameter)
(hash_splat_parameter (identifier) @local.definition.variable.parameter)
(lambda_parameters (identifier) @local.definition.variable.parameter)
(method_parameters (identifier) @local.definition.variable.parameter)
(splat_parameter (identifier) @local.definition.variable.parameter)
(keyword_parameter name: (identifier) @local.definition.variable.parameter)
(optional_parameter name: (identifier) @local.definition.variable.parameter)

(identifier) @local.reference
