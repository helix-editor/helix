; Parse the contents of tagged template literals using
; a language inferred from the tag.

(call_expression
  function: [
    (identifier) @injection.language
    (member_expression
      property: (property_identifier) @injection.language)
  ]
  arguments: (template_string) @injection.content
  (#any-of? @injection.language "html" "css" "json" "sql" "js" "ts" "bash"))

; Parse the contents of $ template literals as shell commands

(call_expression
  function: [
    (identifier) @_template_function_name
    (member_expression
      property: (property_identifier) @_template_function_name)
  ]
  arguments: (template_string) @injection.content
 (#eq? @_template_function_name "$")
 (#set! injection.language "bash"))

; Parse the contents of gql template literals

((call_expression
   function: (identifier) @_template_function_name
   arguments: (template_string) @injection.content)
 (#eq? @_template_function_name "gql")
 (#set! injection.language "graphql"))

; Parse regex syntax within regex literals

((regex_pattern) @injection.content
 (#set! injection.language "regex"))

; Parse JSDoc annotations in multiline comments

((comment) @injection.content
 (#set! injection.language "jsdoc")
 (#match? @injection.content "^/\\*+"))

; Parse general tags in single line comments

((comment) @injection.content
 (#set! injection.language "comment")
 (#match? @injection.content "^//"))

; Match string literals passed to standard browser API methods that expects a
; css selector as argument.
; - https://developer.mozilla.org/en-US/docs/Web/API/Document/querySelector
; - https://developer.mozilla.org/en-US/docs/Web/API/Document/querySelectorAll
; - https://developer.mozilla.org/en-US/docs/Web/API/Element/closest
; - https://developer.mozilla.org/en-US/docs/Web/API/Element/matches
; e.g.
; `const el = document.querySelector("div.user-panel.main input[name='login']");`
(call_expression
  function: (member_expression
    object: (identifier) @_object
    property: (property_identifier) @_property (#any-of? @_property "querySelector" "querySelectorAll" "closest" "matches"))
  arguments: (arguments
               (string (string_fragment) @injection.content))
  (#set! injection.language "css"))
