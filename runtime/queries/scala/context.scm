(object_definition) @context

(class_definition
  class_parameters: (_) @context.params
) @context

(trait_definition
  class_parameters: (_) @context.params
) @context

(enum_definition
  class_parameters: (_) @context.params
) @context

(given_definition
  parameters: (_) @context.params
) @context

(extension_definition
  parameters: (_) @context.params
) @context

(function_definition
  parameters: (_) @context.params
) @context

(if_expression
  alternative: (_) @context
) @context

[
  (call_expression)
  (case_clause)
  (catch_clause)
  (lambda_expression)
  (match_expression)
  (try_expression)
  (while_expression)
] @context


