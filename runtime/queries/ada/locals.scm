(compilation) @local.scope
(package_declaration) @local.scope
(package_body) @local.scope
(subprogram_declaration) @local.scope
(subprogram_body) @local.scope
(block_statement) @local.scope

(with_clause (_) @local.definition)
(procedure_specification name: (_) @local.definition)
(function_specification name: (_) @local.definition)
(package_declaration name: (_) @local.definition)
(package_body name: (_) @local.definition)
(generic_instantiation . name: (_) @local.definition)
(component_declaration . (identifier) @local.definition)
(exception_declaration . (identifier) @local.definition)
(formal_object_declaration . (identifier) @local.definition)
(object_declaration . (identifier) @local.definition)
(parameter_specification . (identifier) @local.definition)
(full_type_declaration . (identifier) @local.definition)
(private_type_declaration . (identifier) @local.definition)
(private_extension_declaration . (identifier) @local.definition)
(incomplete_type_declaration . (identifier) @local.definition)
(protected_type_declaration . (identifier) @local.definition)
(formal_complete_type_declaration . (identifier) @local.definition)
(formal_incomplete_type_declaration . (identifier) @local.definition)
(task_type_declaration . (identifier) @local.definition)
(subtype_declaration . (identifier) @local.definition)

(identifier) @local.reference
