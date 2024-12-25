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
  (#set! "scope" "all")
)

(while_statement
  body: _ @indent
  (#not-kind-eq? @indent block)
  (#set! "scope" "all")
)

(do_statement
  body: _ @indent
  (#not-kind-eq? @indent block)
  (#set! "scope" "all")
)

(if_statement
  consequence: _ @indent
  (#not-kind-eq? @indent block)
  (#set! "scope" "all")
)
(if_statement
  alternative: _ @indent
  (#not-kind-eq? @indent if_statement)
  (#not-kind-eq? @indent block)
  (#set! "scope" "all")
)
(if_statement
  "else" @else
  alternative: (if_statement) @indent
  (#not-same-line? @indent @else)
  (#set! "scope" "all")
)

(if_element
  consequence: _ @indent
  (#set! "scope" "all")
)
(if_element
  alternative: _ @indent
  (#not-kind-eq? @indent if_element)
  (#set! "scope" "all")
)
(if_element
  "else" @else
  alternative: (if_element) @indent
  (#not-same-line? @indent @else)
  (#set! "scope" "all")
)

(for_element
  body: _ @indent
  (#set! "scope" "all")
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

