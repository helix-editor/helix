; Credits to nvim-treesitter/nvim-treesitter-context

(function_definition
  (_ parameters: (_) @context.params)
) @context

[
  (preproc_if)
  (preproc_ifdef)
  (for_statement)
  (if_statement)
  (while_statement)
  (do_statement)
  (struct_specifier)
  (enum_specifier)
] @context
