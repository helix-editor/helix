; https://tree-sitter.github.io/tree-sitter/4-code-navigation.html
;
; The Nix data model is "everything is an attrset of expressions";
; functions and values share the same binding form. We capture
; definitions broadly and mark function-valued bindings as
; @definition.function specifically so code navigation can distinguish them.

; ----- Definitions -----

; Generic binding - any top-level or nested attrset attribute.
; Anchor @name to the leaf of the attrpath so `foo.bar.baz = ...`
; tags `baz` rather than the whole dotted path.
(binding
  attrpath: (attrpath
    attr: (identifier) @name .)
  expression: (_)) @definition.constant

; Function-valued bindings - `foo = x: ...`.
(binding
  attrpath: (attrpath
    attr: (identifier) @name .)
  expression: (function_expression)) @definition.function

; inherit - definitions brought from another scope.
(inherit
  attrs: (inherited_attrs attr: (identifier) @name)) @definition.constant

(inherit_from
  attrs: (inherited_attrs attr: (identifier) @name)) @definition.constant

; ----- References -----

; Any bare identifier used as a value expression is a reference.
(variable_expression
  name: (identifier) @name) @reference.constant

; Function application - the thing being called is a reference.call.
(apply_expression
  function: (variable_expression
    name: (identifier) @name)) @reference.call

; Method-style call: `foo.bar.baz arg` - tag the leaf attrname.
(apply_expression
  function: (select_expression
    attrpath: (attrpath
      attr: (identifier) @name .))) @reference.call

; Attribute access (non-call) - `foo.bar.baz` lookup.
(select_expression
  attrpath: (attrpath
    attr: (identifier) @name .)) @reference.constant
