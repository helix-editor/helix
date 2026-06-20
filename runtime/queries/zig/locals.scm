; Zig has no separate parameter/capture *use* coloring without scope tracking:
; highlights.scm colors the binding, locals resolve the references. Only
; parameters and `|capture|` payloads are defined here — `const`/`var` names are
; left to highlights.scm because Zig reuses `const Name = struct/enum/error{…}`
; for type definitions, which must stay @type, not resolve to a variable.

[
  (function_declaration)
  (block)
  (for_statement)
  (while_statement)
  (if_statement)
  (catch_expression)
  (switch_case)
] @local.scope

(parameter name: (identifier) @local.definition.variable.parameter)
(payload (identifier) @local.definition.variable.parameter)

(identifier) @local.reference
