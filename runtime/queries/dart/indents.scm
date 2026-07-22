; things surrounded by ({[]})
[
  (template_substitution)
  (list_literal)
  (set_or_map_literal)
  (parenthesized_expression)
  (arguments)
  (index_selector)
  (block)
  (assertion_arguments)
  (switch_block)
  ; statements after a `case`/`default` label (the node also holds the label,
  ; so the default tail scope indents only the body lines, not the label)
  (switch_statement_case)
  (switch_statement_default)
  (catch_parameters)
  (for_loop_parts)
  (configuration_uri_condition)
  (enum_body)
  (class_body)
  (extension_body)
  (parameter_type_list)
  (optional_positional_parameter_types)
  (named_parameter_types)
  (formal_parameter_list)
  (optional_formal_parameters)
] @indent

; control flow statement which accept one line as body

(for_statement
  body: _ @indent
  (#not-kind-eq? @indent block)

)

(while_statement
  body: _ @indent
  (#not-kind-eq? @indent block)

)

(do_statement
  body: _ @indent
  (#not-kind-eq? @indent block)

)

(if_statement
  consequence: _ @indent
  (#not-kind-eq? @indent block)

)
(if_statement
  alternative: _ @indent
  (#not-kind-eq? @indent if_statement)
  (#not-kind-eq? @indent block)

)
(if_statement
  "else" @else
  alternative: (if_statement) @indent
  (#not-same-line? @indent @else)

)

(if_element
  consequence: _ @indent

)
(if_element
  alternative: _ @indent
  (#not-kind-eq? @indent if_element)

)
(if_element
  "else" @else
  alternative: (if_element) @indent
  (#not-same-line? @indent @else)

)

(for_element
  body: _ @indent

)

; simple statements
[
  (local_variable_declaration)
  (break_statement)
  (continue_statement)
  (return_statement)
  (yield_statement)
  (yield_each_statement)
  (expression_statement)
] @indent

[
  "}"
  "]"
  ")"
] @outdent

(string_literal) @opaque
