((comment) @injection.content
 (#set! injection.language "comment")
 (#set! injection.include-children))

; string.format("format string", ...)
((function_call
  name: (dot_index_expression
    table: (identifier) @_table
    field: (identifier) @_function)
  arguments: (arguments
    .
    (string
      content: (string_content) @injection.content)))
  (#eq? @_table "string")
  (#eq? @_function "format")
  (#set! injection.language "lua-format-string"))

; ("format"):format(...)
((function_call
  name: (method_index_expression
    table: (parenthesized_expression
      (string
        content: (string_content) @injection.content))
    method: (identifier) @_function))
  (#eq? @_function "format")
  (#set! injection.language "lua-format-string"))
