; Elixir is macro-heavy and `def`/`defp` are ordinary `call`s, so a general
; def/scope story can't be expressed without guessing. The one unambiguous
; binding form is the anonymous function `fn ... -> ... end`.

; Scopes

(anonymous_function) @local.scope

; Definitions

; `fn x, y -> ... end`: plain identifiers in the clause head are parameters.
; (Destructuring patterns are intentionally left to highlights.scm.)
(stab_clause
  left: (arguments
    (identifier) @local.definition.variable.parameter))

; `fn x when guard -> ... end`: the head is wrapped in a `when` binary_operator.
(stab_clause
  left: (binary_operator
    left: (arguments
      (identifier) @local.definition.variable.parameter)))

; References

(identifier) @local.reference
