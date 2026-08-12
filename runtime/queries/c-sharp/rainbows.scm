[
  (accessor_list)
  (anonymous_object_creation_expression)
  (argument_list)
  (array_rank_specifier)
  (attribute_argument_list)
  (attribute_list)
  (block)
  (bracketed_argument_list)
  (bracketed_parameter_list)
  (calling_convention)
  (cast_expression)
  (catch_declaration)
  (catch_filter_clause)
  (checked_expression)
  (collection_expression)
  (declaration_list)
  (default_expression)
  (do_statement)
  (enum_member_declaration_list)
  (fixed_statement)
  (global_attribute)
  (if_statement)
  (implicit_array_creation_expression)
  (implicit_stackalloc_expression)
  (initializer_expression)
  (list_pattern)
  (lock_statement)
  (makeref_expression)
  (parameter_list)
  (parenthesized_expression)
  (parenthesized_pattern)
  (parenthesized_variable_designation)
  (positional_pattern_clause)
  (preproc_line)
  (property_pattern_clause)
  (reftype_expression)
  (refvalue_expression)
  (sizeof_expression)
  (switch_body)
  (switch_statement)
  (tuple_expression)
  (tuple_pattern)
  (tuple_type)
  (type_parameter_constraint)
  (typeof_expression)
  (using_statement)
  (while_statement)
] @rainbow.scope

(type_argument_list ["<" ">"] @rainbow.bracket) @rainbow.scope
(type_parameter_list ["<" ">"] @rainbow.bracket) @rainbow.scope

[
  "(" ")"
  "{" "}"
  "[" "]"
] @rainbow.bracket
