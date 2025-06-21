((method_def) @local.scope
 (#set! local.scope-inherits false))
((fun_def) @local.scope
 (#set! local.scope-inherits false))

(block) @local.scope

(param
  name: (identifier) @local.definition.variable.parameter)

(assign
  lhs: (identifier) @local.definition.variable)

(identifier) @local.reference
