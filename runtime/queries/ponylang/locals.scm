[
  (entity)
  (method)
  (behavior)
  (constructor)
  ("if")
  (elseif)
  (ifdef)
  (elseifdef)
  (iftype)
  (elseiftype)
  (match)
  (match_case)
  ("while")
  ("repeat")
  ("for")
  (lambda)
  (try_block)
  (with)
] @local.scope
(match else_block: (block) @local.scope)
(try_block else_block: (block) @local.scope)
(try_block then_block: (block) @local.scope)
(with else_block: (block) @local.scope)

(param name: (identifier) @local.definition.variable.parameter)
(lambdaparam name: (identifier) @local.definition.variable.parameter)

; only lower case identifiers are references
(
  (identifier) @local.reference
  (#match? @local.reference "^[a-z_][a-zA-Z_]*")
)
