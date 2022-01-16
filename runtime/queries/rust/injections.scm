([(line_comment) (block_comment)] @injection.content
 (#set! injection.language "comment"))

((macro_rule
  (token_tree) @injection.content)
 (#set! injection.language "rust")
 (#set! injection.include-children))

; inline_c::assert_c
((macro_invocation
  macro: (identifier) @_c (#eq? @_c "assert_c")
  (token_tree) @injection.content)
 (#set! injection.language "c")
 (#set! injection.include-children))

; inline_c::assert_cxx
((macro_invocation
  macro: (identifier) @_cxx (#eq? @_cxx "assert_cxx")
  (token_tree) @injection.content)
 (#set! injection.language "cpp")
 (#set! injection.include-children))

; typed_html::html
((macro_invocation
  macro: (identifier) @_html (#eq? @_html "html")
  (token_tree) @injection.content)
 (#set! injection.language "html")
 (#set! injection.include-children))

; inline_python::python
; TODO: fix bug where first import is not highlighted
((macro_invocation
  macro: (scoped_identifier
    name: (identifier) @_python (#eq? @_python "python"))
  (token_tree) @injection.content)
 (#set! injection.language "python")
 (#set! injection.include-children))

((macro_invocation
  macro: (identifier) @_python (#eq? @_python "python")
  (token_tree) @injection.content)
 (#set! injection.language "python")
 (#set! injection.include-children))

; embed_js::inline_js
((macro_invocation
  macro: (identifier) @_js (#eq? @_js "include_js")
  (token_tree) @injection.content)
 (#set! injection.language "javascript")
 (#set! injection.include-children))

(call_expression
  function: (scoped_identifier
    path: (identifier) @_regex (#eq? @_regex "Regex")
    name: (identifier) @_new (#eq? @_new "new"))
  arguments: (arguments (raw_string_literal) @injection.content)
  (#set! injection.language "regex"))

(call_expression
  function: (scoped_identifier
    path: (scoped_identifier (identifier) @_regex (#eq? @_regex "Regex") .)
    name: (identifier) @_new (#eq? @_new "new"))
  arguments: (arguments (raw_string_literal) @injection.content)
  (#set! injection.language "regex"))
