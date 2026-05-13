; Scopes

[
  (structure)
  (function_definition)
  (anonymous_lambda_expression)
  (named_lambda_expression)
  (block_expression)
] @local.scope

; Definitions

(function_definition (function_identifier) @local.definition)
(struct_constructor_declaration (lowercase_identifier) @local.definition)

(value_definition (lowercase_identifier) @local.definition)
(positional_parameter (lowercase_identifier) @local.definition)
(labelled_parameter (label (lowercase_identifier)) @local.definition)
(optional_parameter (optional_label (lowercase_identifier)) @local.definition)
(optional_parameter_with_default (label (lowercase_identifier)) @local.definition)
(let_mut_expression (lowercase_identifier) @local.definition)

(struct_definition (identifier) @local.definition)
(enum_definition (identifier) @local.definition)
(type_definition (identifier) @local.definition)

; References

(qualified_identifier) @local.reference
(qualified_type_identifier) @local.reference
