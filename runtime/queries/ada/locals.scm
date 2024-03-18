;;  Better highlighting by referencing to the definition, for variable references.
;;  See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#local-variables

(compilation) @local.scope
(package_declaration) @local.scope
(package_body) @local.scope
(subprogram_declaration) @local.scope
(subprogram_body) @local.scope
(block_statement) @local.scope

(with_clause (_) @local.definition.import)
(procedure_specification name: (_) @local.definition.function)
(function_specification name: (_) @local.definition.function)
(package_declaration name: (_) @local.definition.var)
(package_body name: (_) @local.definition.var)
(generic_instantiation . name: (_) @local.definition.var)
(component_declaration . (identifier) @local.definition.var)
(exception_declaration . (identifier) @local.definition.var)
(formal_object_declaration . (identifier) @local.definition.var)
(object_declaration . (identifier) @local.definition.var)
(parameter_specification . (identifier) @local.definition.var)
(full_type_declaration . (identifier) @local.definition.type)
(private_type_declaration . (identifier) @local.definition.type)
(private_extension_declaration . (identifier) @local.definition.type)
(incomplete_type_declaration . (identifier) @local.definition.type)
(protected_type_declaration . (identifier) @local.definition.type)
(formal_complete_type_declaration . (identifier) @local.definition.type)
(formal_incomplete_type_declaration . (identifier) @local.definition.type)
(task_type_declaration . (identifier) @local.definition.type)
(subtype_declaration . (identifier) @local.definition.type)

(identifier) @local.reference
