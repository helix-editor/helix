((comment) @injection.content
  (#set! injection.language "comment"))

([
    (empty_statement)
    (expression_statement)
    (function_declaration)
    (generator_function_declaration)
    (statement_block)
    (switch_statement)
    (try_statement)
    (variable_declaration)
    (with_statement)
  ] @injection.content
  (#set! injection.include-children)
  (#set! injection.language "javascript"))
