((comment) @injection.content
 (#set! injection.language "comment"))

; string.format("format string", ...)
((call_stmt
  invoked: (var
    table_name: (name) @_table
    (key
      field_name: (name) @_function))
  (arglist
    . (string) @injection.content))
  (#eq? @_table "string")
  (#eq? @_function "format")
  (#set! injection.language "lua-format-string"))

; ("format"):format(...)
((call_stmt
  method_table: (exp_wrap (string) @injection.content)
  method_name: (name) @_function)
  (#eq? @_function "format")
  (#set! injection.language "lua-format-string"))
