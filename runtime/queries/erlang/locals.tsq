; Specs and Callbacks
(attribute
  (stab_clause
    pattern: (arguments (variable)? @local.definition)
    ; If a spec uses a variable as the return type (and later a `when` clause to type it):
    body: (variable)? @local.definition)) @local.scope

; parametric `-type`s
((attribute
    name: (atom) @_type
    (arguments
      (binary_operator
        left: (call (arguments (variable) @local.definition))
        operator: "::") @local.scope))
 (#match? @_type "(type|opaque)"))

; macros
((attribute
   name: (atom) @_define
   (arguments
     (call (arguments (variable) @local.definition)))) @local.scope
 (#eq? @_define "define"))

; `fun`s
(anonymous_function (stab_clause pattern: (arguments (variable) @local.definition))) @local.scope

; Ordinary functions
(function_clause pattern: (arguments (variable) @local.definition)) @local.scope

(variable) @local.reference
