; Upstream: https://github.com/alex-pinkus/tree-sitter-swift/blob/57c1c6d6ffa1c44b330182d41717e6fe37430704/queries/locals.scm
(import_declaration (identifier) @definition.import)
(function_declaration name: (simple_identifier) @definition.function)

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
