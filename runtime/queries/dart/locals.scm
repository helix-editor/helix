; Scopes
;-------

[
 (block)
 (try_statement)
 (catch_clause)
 (finally_clause)
] @local.scope

; Definitions
;------------

(class_definition
 body: (_) @local.definition)

; References
;------------

(identifier) @local.reference
