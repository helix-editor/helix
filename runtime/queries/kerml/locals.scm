; Scopes

[
  (source_file)
  (body)
  (body_expression)
] @local.scope

; Definitions

(package name: (_) @local.definition.namespace)
(library_package name: (_) @local.definition.namespace)
(namespace name: (_) @local.definition.namespace)
(type name: (_) @local.definition.type)
(classifier name: (_) @local.definition.type)
(data_type name: (_) @local.definition.type)
(class name: (_) @local.definition.type)
(structure name: (_) @local.definition.type)
(association name: (_) @local.definition.type)
(association_structure name: (_) @local.definition.type)
(metaclass name: (_) @local.definition.type)
(interaction name: (_) @local.definition.type)

(behavior name: (_) @local.definition.function)
(function name: (_) @local.definition.function)
(predicate name: (_) @local.definition.function)
(step name: (_) @local.definition.function)
(expression_feature name: (_) @local.definition.function)
(boolean_expression name: (_) @local.definition.function)
(invariant name: (_) @local.definition.label)

(feature name: (_) @local.definition.variable)
(connector name: (_) @local.definition.variable)
(binding_connector name: (_) @local.definition.variable)
(succession name: (_) @local.definition.variable)
(flow name: (_) @local.definition.variable)
(succession_flow name: (_) @local.definition.variable)
(metadata_feature name: (_) @local.definition.variable)
(alias_member name: (_) @local.definition.namespace)

; References

(qualified_name [(identifier) (unrestricted_name)] @local.reference)

; Named arguments are parameter names, not variable references.
(named_argument name: (qualified_name [(identifier) (unrestricted_name)] @_))
