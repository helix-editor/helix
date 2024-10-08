[
  (if_statement)
  (for_statement)
  (while_statement)
  (match_statement)
  (pattern_section)

  (function_definition)
  (lambda)
  (constructor_definition)
  (class_definition)
  (enum_definition)

  (dictionary (_))
  (array (_))
  (setget)
] @indent

[
  (if_statement)
  (for_statement)
  (while_statement)
  (match_statement)
  (pattern_section)

  (function_definition)
  (class_definition)
] @extend

[
  (return_statement)
  (break_statement)
  (continue_statement)
  (pass_statement)
] @extend.prevent-once

