; Credits to nvim-treesitter/nvim-treesitter-context

(function_declaration
	(parameter_list) @context.params
) @context

(method_declaration
	(parameter_list) @context.params
) @context

(for_statement
	(_)) @context

[
  (const_declaration)
	(for_statement)
	(if_statement)
  (import_declaration)
  (type_declaration)
  (var_declaration)
] @context
