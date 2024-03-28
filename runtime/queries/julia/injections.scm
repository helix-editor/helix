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
      (short_function_definition)
      (assignment)
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
  [
    (command_literal)
    (prefixed_command_literal)
  ] @injection.content
  (#set! injection.language "sh"))

(
  (prefixed_string_literal
    prefix: (identifier) @function.macro) @injection.content
  (#eq? @function.macro "r")
  (#set! injection.language "regex"))

(
  (prefixed_string_literal
    prefix: (identifier) @function.macro) @injection.content
  (#eq? @function.macro "md")
  (#set! injection.language "markdown"))
