(import_declaration (identifier) @definition.import)
(function_declaration name: (simple_identifier) @definition.function)

; Scopes
[
 (statements)
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
] @scope
