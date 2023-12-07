;;  Better highlighting by referencing to the definition, for variable
;;  references. However, this is not yet supported by neovim
;;  See https://tree-sitter.github.io/tree-sitter/syntax-highlighting#local-variables

(compilation) @scope
(package_declaration) @scope
(package_body) @scope
(subprogram_declaration) @scope
(subprogram_body) @scope
(block_statement) @scope

(with_clause (_) @definition.import)
(procedure_specification name: (_) @definition.function)
(function_specification name: (_) @definition.function)
(package_declaration name: (_) @definition.var)
(package_body name: (_) @definition.var)
(generic_instantiation . name: (_) @definition.var)
(component_declaration . (identifier) @definition.var)
(exception_declaration . (identifier) @definition.var)
(formal_object_declaration . (identifier) @definition.var)
(object_declaration . (identifier) @definition.var)
(parameter_specification . (identifier) @definition.var)
(full_type_declaration . (identifier) @definition.type)
(private_type_declaration . (identifier) @definition.type)
(private_extension_declaration . (identifier) @definition.type)
(incomplete_type_declaration . (identifier) @definition.type)
(protected_type_declaration . (identifier) @definition.type)
(formal_complete_type_declaration . (identifier) @definition.type)
(formal_incomplete_type_declaration . (identifier) @definition.type)
(task_type_declaration . (identifier) @definition.type)
(subtype_declaration . (identifier) @definition.type)

(identifier) @reference
