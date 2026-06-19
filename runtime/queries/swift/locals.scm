; Upstream: https://github.com/alex-pinkus/tree-sitter-swift/blob/57c1c6d6ffa1c44b330182d41717e6fe37430704/queries/locals.scm
(import_declaration (identifier) @local.definition.namespace)
(function_declaration name: (simple_identifier) @local.definition.function)

; Parameters: the `name` field is the in-body binding (external_name is the
; call-site label, handled as a discard below).
(parameter name: (simple_identifier) @local.definition.variable.parameter)
(lambda_parameter name: (simple_identifier) @local.definition.variable.parameter)

; Scopes
[
 (for_statement)
 (while_statement)
 (repeat_while_statement)
 (do_statement)
 (if_statement)
 (guard_statement)
 (switch_statement)
 (property_declaration)
 (function_declaration)
 (class_declaration)
 (protocol_declaration)
 (lambda_literal)
] @local.scope

(simple_identifier) @local.reference

; Discards: identifiers that look like references but aren't variable uses.
(call_expression (simple_identifier) @_) ; foo() call name
(navigation_suffix (simple_identifier) @_) ; .bar member/method name
(parameter external_name: (simple_identifier) @_) ; call-site argument label
