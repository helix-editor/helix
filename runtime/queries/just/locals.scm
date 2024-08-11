; This file tells us about the scope of variables so e.g. local
; variables override global functions with the same name

; Scope

(recipe) @local.scope

; Definitions

(alias
  name: (identifier) @local.definition)

(assignment
  name: (identifier) @local.definition)

(mod
  name: (identifier) @local.definition)

(recipe_parameter
  name: (identifier) @local.definition)

(recipe
  name: (identifier) @local.definition)

; References

(alias
  name: (identifier) @local.reference)

(function_call
  name: (identifier) @local.reference)

(recipe_dependency
  name: (identifier) @local.reference)

(value
  (identifier) @local.reference)
