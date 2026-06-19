; Variable scopes, definitions and references for Nix.
;
; Nix `let` / `rec` bindings are recursive and lazily evaluated, so a reference
; can resolve to a binding that appears later in source order. tree-sitter's
; locals resolver is order-sensitive, so a forward reference simply keeps its
; base `@variable` highlight instead of the definition's class — a cosmetic
; limitation, never a miscolour.

; Scopes

[
  (function_expression)
  (let_expression)
  (rec_attrset_expression)
] @local.scope

; Definitions

(formal
  name: (identifier) @local.definition.variable.parameter)

; `@args:` binds the whole argument set.
(function_expression
  universal: (identifier) @local.definition.variable.parameter)

; `let name = ...;` bindings. Dotted attrpaths aren't plain locals, so restrict
; to a single-attr path.
(let_expression
  (binding_set
    (binding
      attrpath: (attrpath
        attr: (identifier) @local.definition.variable) .)))

; References

; Only `variable_expression` names are variable references; a bare `identifier`
; elsewhere (attrpath members, binding names) is attribute access, not a local.
(variable_expression
  name: (identifier) @local.reference)

; Discard: a variable in call position (`f x`) is highlighted `@function` by
; highlights.scm. Cancel its reference resolution here so locals doesn't recolour
; it as a plain variable.
(apply_expression
  function: (variable_expression
    name: (identifier) @_))
