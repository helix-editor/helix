; Credits to nvim-treesitter-context

(interface_declaration) @context

(class_declaration) @context

(enum_declaration) @context

(struct_declaration) @context

(record_declaration) @context

(namespace_declaration) @context

(constructor_declaration
    (parameter_list) @context.params
  ) @context

(destructor_declaration
    (parameter_list) @context.params
  ) @context

(method_declaration
    (parameter_list) @context.params
  ) @context

(switch_statement) @context

(for_statement) @context

(if_statement) @context

([
  (do_statement)
  (while_statement)
] @context)

(try_statement) @context

(catch_clause) @context

(finally_clause) @context

