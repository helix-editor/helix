;; Harneet Programming Language - Local Scope Queries for Helix Editor
;; Tree-sitter locals queries for semantic highlighting and navigation

;; Scopes
(block) @local.scope
(function_declaration) @local.scope
(for_statement) @local.scope
(if_statement) @local.scope

;; Definitions
(var_declaration name: (identifier) @local.definition.var)
(short_var_declaration left: (identifier) @local.definition.var)
(function_declaration name: (identifier) @local.definition.function)
(parameter name: (identifier) @local.definition.parameter)

;; References  
(identifier) @local.reference

;; Import definitions
(import_spec name: (identifier) @local.definition.import)
(import_spec alias: (identifier) @local.definition.import)

;; Method definitions (for future struct support)
(method_declaration name: (identifier) @local.definition.method)
(method_declaration receiver: (parameter name: (identifier) @local.definition.parameter))

;; Type definitions (for future type system)
(type_declaration name: (identifier) @local.definition.type)