; From <https://github.com/IndianBoy42/tree-sitter-just/blob/6c2f018ab1d90946c0ce029bb2f7d57f56895dff/queries-flavored/helix/locals.scm>
;
; This file tells us about the scope of variables so e.g. local
; variables override global functions with the same name

; Scope

(recipe) @local.scope

; Definitions

(alias
  left: (identifier) @local.definition)

(assignment
  left: (identifier) @local.definition)

(module
  name: (identifier) @local.definition)

(parameter
  name: (identifier) @local.definition)

(recipe_header
  name: (identifier) @local.definition)

; References

(alias
  right: (identifier) @local.reference)

(function_call
  name: (identifier) @local.reference)

(dependency
  name: (identifier) @local.reference)

(dependency_expression
  name: (identifier) @local.reference)

(value
  (identifier) @local.reference)
