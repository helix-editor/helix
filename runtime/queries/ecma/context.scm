(arrow_function
	(formal_parameters) @context.params
) @context

(function_declaration
	(formal_parameters) @context.params
) @context

(function
	(formal_parameters) @context.params
) @context

(generator_function_declaration
	(formal_parameters) @context.params
) @context

[
	(call_expression)
	(class_declaration)
  (else_clause)
  (for_statement)
  ; (interface_declaration) ; not usable in javascript
  (lexical_declaration)
  (method_definition)
  (object)
  (pair)
  (while_statement)
	(switch_statement)
	(switch_case)
] @context

