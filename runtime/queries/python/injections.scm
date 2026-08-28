((comment) @injection.content
 (#set! injection.language "comment"))

; Match all 9 functions in the `re` module from the standard library that
; that takes a regex pattern as first argument.
; https://docs.python.org/3/library/re.html#functions
(call
  function: (attribute
    object: (identifier) @_module (#eq? @_module "re")
    attribute: (identifier) @_function (#any-of? @_function "compile" "search" "match" "fullmatch" "sub" "subn" "findall" "finditer" "split"))
  arguments: (argument_list
    . (string
        (string_content) @injection.content))
  (#set! injection.language "regex"))
