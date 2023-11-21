; inherits: c

(reference_declarator
  (identifier) @definition.var)

(type_parameter_declaration
  (type_identifier) @definition.type)
(template_declaration) @scope

(template_function
  name: (identifier) @definition.function) @scope

[
 (foreach_statement)
 (foreach_instance_statement)
 (unmasked_statement)
] @scope
