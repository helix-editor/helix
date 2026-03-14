(compilation) @local.scope
(package_declaration) @local.scope
(package_body) @local.scope
(subprogram_declaration) @local.scope
(subprogram_body) @local.scope
(block_statement) @local.scope

(parameter_specification . (identifier) @local.definition.variable.parameter)

(identifier) @local.reference
