; === Tag Names ===

; Tags that start with a lower case letter are HTML tags
; We'll also use this highlighting for named blocks (which start with `:`)
((tag_name) @tag
  (#match? @tag "^(:)?[a-z]"))
; Tags that start with a capital letter are Glimmer components
((tag_name) @constructor
  (#match? @constructor "^[A-Z]"))

(attribute_name) @attribute

(string_literal) @string
(number_literal) @constant.numeric.integer
(boolean_literal) @constant.builtin.boolean

(concat_statement) @string

; === Block Statements ===

; Highlight the brackets
(block_statement_start) @punctuation.delimiter
(block_statement_end) @punctuation.delimiter

; Highlight `if`/`each`/`let`
(block_statement_start path: (identifier) @keyword.control.conditional)
(block_statement_end path: (identifier) @keyword.control.conditional)
((mustache_statement (identifier) @keyword.control.conditional)
 (#eq? @keyword.control.conditional "else"))

; == Mustache Statements ===

; Hightlight the whole statement, to color brackets and separators
(mustache_statement) @punctuation.delimiter

; An identifier in a mustache expression is a variable
((mustache_statement [
  (path_expression (identifier) @variable)
  (identifier) @variable
  ])
  (#not-any-of? @variable "yield" "outlet" "this" "else"))
; As are arguments in a block statement
((block_statement_start argument: [
  (path_expression (identifier) @variable)
  (identifier) @variable
  ])
 (#not-eq? @variable "this"))
; As is an identifier in a block param
(block_params (identifier) @variable)
; As are helper arguments
((helper_invocation argument: [
  (path_expression (identifier) @variable)
  (identifier) @variable
  ])
  (#not-eq? @variable "this"))
; `this` should be highlighted as a built-in variable
((identifier) @variable.builtin
  (#eq? @variable.builtin "this"))

; If the identifier is just "yield" or "outlet", it's a keyword
((mustache_statement (identifier) @keyword.control.return)
  (#any-of? @keyword.control.return "yield" "outlet"))

; Helpers are functions
((helper_invocation helper: [
  (path_expression (identifier) @function)
  (identifier) @function
  ])
  (#not-any-of? @function "if" "yield"))

((helper_invocation helper: (identifier) @keyword.control.conditional)
  (#any-of? @keyword.control.conditional "if" "yield"))

(hash_pair key: (identifier) @variable)
(hash_pair value: (identifier) @variable)
(hash_pair [
  (path_expression (identifier) @variable)
  (identifier) @variable
  ])

(comment_statement) @comment

(attribute_node "=" @operator)

(block_params "as" @keyword.control)
(block_params "|" @operator)

[
  "<"
  ">"
  "</"
  "/>"
] @punctuation.delimiter

