; Scopes, definitions and references for Xo.

[
  (source_file)
  (function_declaration)
  (function_literal)
  (block)
  (match_arm)
  (for_statement)
  (while_statement)
  (if_expression)
] @local.scope

(let_statement pattern: (identifier) @local.definition)
(var_statement pattern: (identifier) @local.definition)
(let_statement pattern: (tuple_pattern (identifier) @local.definition))
(var_statement pattern: (tuple_pattern (identifier) @local.definition))
(for_statement pattern: (identifier) @local.definition)
(for_statement pattern: (tuple_pattern (identifier) @local.definition))
(parameter name: (identifier) @local.definition)
(closure_parameter name: (identifier) @local.definition)
(const_declaration name: (identifier) @local.definition)
(function_declaration name: (identifier) @local.definition)
(match_arm pattern: (identifier) @local.definition)
(variant_pattern (pattern_field (identifier) @local.definition))
(struct_declaration name: (identifier) @local.definition)
(enum_declaration name: (identifier) @local.definition)
(interface_declaration name: (identifier) @local.definition)
(type_declaration name: (identifier) @local.definition)

(identifier) @local.reference
