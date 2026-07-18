; Scopes

[
  (structure)
  (function_definition)
  (anonymous_lambda_expression)
  (named_lambda_expression)
  (block_expression)
] @local.scope

; Definitions

(value_definition (lowercase_identifier) @local.definition.variable)
(let_expression (lowercase_identifier) @local.definition.variable)
(letrec_expression (lowercase_identifier) @local.definition.variable)
(and_expression (lowercase_identifier) @local.definition.variable)
(guard_let_expression (lowercase_identifier) @local.definition.variable)
(let_mut_expression (lowercase_identifier) @local.definition.variable.mutable)

(positional_parameter (lowercase_identifier) @local.definition.variable.parameter)
(labelled_parameter (label (lowercase_identifier)) @local.definition.variable.parameter)
(optional_parameter (optional_label (lowercase_identifier)) @local.definition.variable.parameter)
(optional_parameter_with_default (label (lowercase_identifier)) @local.definition.variable.parameter)

((positional_parameter (lowercase_identifier) @local.definition.variable.builtin)
 (#eq? @local.definition.variable.builtin "self"))
((labelled_parameter (label (lowercase_identifier)) @local.definition.variable.builtin)
 (#eq? @local.definition.variable.builtin "self"))
((optional_parameter (optional_label (lowercase_identifier)) @local.definition.variable.builtin)
 (#eq? @local.definition.variable.builtin "self"))
((optional_parameter_with_default (label (lowercase_identifier)) @local.definition.variable.builtin)
 (#eq? @local.definition.variable.builtin "self"))

; References

(qualified_identifier) @local.reference
