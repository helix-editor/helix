(knot_block) @local.scope
(stitch_block) @local.scope

(param (identifier) @local.definition.variable.parameter)
(param (divert (identifier) @local.definition.variable.parameter))
(temp_def name: (identifier) @local.definition.variable)

(identifier) @local.reference
(divert (identifier) @local.reference)
