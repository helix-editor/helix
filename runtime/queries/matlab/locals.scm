(function_definition name: (identifier) @local.definition ?) @local.scope
(function_arguments (identifier)* @local.definition)

(lambda (arguments (identifier) @local.definition)) @local.scope

(assignment left: ((function_call
                     name: (identifier) @local.definition)))
(assignment left: ((field_expression . [(function_call
                                          name: (identifier) @local.definition)
                                        (identifier) @local.definition])))
(assignment left: (_) @local.definition)
(assignment (multioutput_variable (_) @local.definition))

(iterator . (identifier) @local.definition)
(global_operator (identifier) @local.definition)
(persistent_operator (identifier) @local.definition)
(catch_clause (identifier) @local.definition)

(identifier) @local.reference
