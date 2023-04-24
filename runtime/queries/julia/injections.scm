(
  (source_file
    (string_literal) @injection.content
    .
    [
      (module_definition)
      (function_definition)
      (macro_definition)
      (primitive_definition)
      (abstract_definition)
      (struct_definition)
      (assignment_expression)
      (const_statement)
    ])
  (#set! injection.language "markdown"))

(
  [
    (line_comment) 
    (block_comment)
  ] @injection.content
  (#set! injection.language "comment"))

(
  (prefixed_string_literal
    prefix: (identifier) @function.macro) @injection.content
  (#eq? @function.macro "re")
  (#set! injection.language "regex"))

(
  (prefixed_string_literal
    prefix: (identifier) @function.macro) @injection.content
  (#eq? @function.macro "md")
  (#set! injection.language "markdown"))
