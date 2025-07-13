(template_body) @local.scope
(lambda_expression) @local.scope


(function_declaration
      name: (identifier) @local.definition.function) @local.scope

(function_definition
      name: (identifier) @local.definition.function)

(parameter
  name: (identifier) @local.definition.variable.parameter)

(identifier) @local.reference
