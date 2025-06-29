; Specs and Callbacks
(attribute
  (stab_clause
    pattern: (arguments (variable)? @local.definition.variable.parameter)
    ; If a spec uses a variable as the return type (and later a `when` clause to type it):
    body: (variable)? @local.definition.variable.parameter)) @local.scope

; parametric `-type`s
((attribute
    name: (atom) @_type
    (arguments
      (binary_operator
        left: (call (arguments (variable) @local.definition.variable.parameter))
        operator: "::") @local.scope))
 (#match? @_type "(type|opaque)"))

; `fun`s
(anonymous_function (stab_clause pattern: (arguments (variable) @local.definition.variable.parameter))) @local.scope

; Ordinary functions
((function_clause
   pattern: (arguments (variable) @local.definition.variable.parameter)) @local.scope
 (#not-match? @local.definition.variable.parameter "^_"))

(variable) @local.reference
