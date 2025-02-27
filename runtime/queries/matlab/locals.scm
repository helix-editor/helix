(function_definition name: (identifier) @local.definition.function ?) @local.scope
(function_arguments (identifier)* @local.definition.variable.parameter)

(lambda (arguments (identifier) @local.definition.variable.parameter)) @local.scope

(identifier) @local.reference
