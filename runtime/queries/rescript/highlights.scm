(comment) @comment

[
  "."
  ","
  "|"
] @punctuation.delimiter

[
  "++"
  "+"
  "+."
  "-"
  "-."
  "*"
  "**"
  "*."
  "/."
  "<="
  "=="
  "==="
  "!"
  "!="
  "!=="
  ">="
  "&&"
  "||"
  "="
  ":="
  "->"
  "|>"
  ":>"
  "+="
  (uncurry)
] @operator

; Explicitly enclose these operators with binary_expression
; to avoid confusion with JSX tag delimiters
(binary_expression ["<" ">" "/"] @operator)

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

(polyvar_type
  [
   "["
   "[>"
   "[<"
   "]"
  ] @punctuation.bracket)

[
  "~"
  "?"
  "=>"
  ".."
  "..."
] @punctuation.special


; Identifiers
;------------

; Escaped identifiers like \"+."
((value_identifier) @function.macro
 (#match? @function.macro "^\\.*$"))

[
  (type_identifier)
  (unit_type)
  (list)
  (list_pattern)
] @type

[
  (variant_identifier)
  (polyvar_identifier)
] @constructor

(record_type_field (property_identifier) @type)
(object_type (field (property_identifier) @type))
(record_field (property_identifier) @variable.other.member)
(object (field (property_identifier) @variable.other.member))
(member_expression (property_identifier) @variable.other.member)
(module_identifier) @namespace

; Parameters
;----------------

(list_pattern (value_identifier) @variable.parameter)
(spread_pattern (value_identifier) @variable.parameter)

; String literals
;----------------

[
  (string)
  (template_string)
] @string

(template_substitution
  "${" @punctuation.bracket
  "}" @punctuation.bracket) @embedded

(character) @string.special
(escape_sequence) @string.special

; Other literals
;---------------

[
  (true)
  (false)
] @constant.builtin

(number) @constant.numeric
(polyvar) @constant
(polyvar_string) @constant

; Functions
;----------

; parameter(s) in parens
[
 (parameter (value_identifier))
 (labeled_parameter (value_identifier))
] @variable.parameter

; single parameter with no parens
(function parameter: (value_identifier) @variable.parameter)

; first-level descructuring (required for nvim-tree-sitter as it only matches direct
; children and the above patterns do not match destructuring patterns in NeoVim)
(parameter (tuple_pattern (tuple_item_pattern (value_identifier) @variable.parameter)))
(parameter (array_pattern (value_identifier) @variable.parameter))
(parameter (record_pattern (value_identifier) @variable.parameter))

; Meta
;-----

(decorator_identifier) @keyword.directive

(extension_identifier) @keyword
("%") @keyword

; Misc
;-----

(subscript_expression index: (string) @attribute)
(polyvar_type_pattern "#" @constant)

[
  ("include")
  ("open")
] @keyword.control.import

[
  "as"
  "export"
  "external"
  "let"
  "module"
  "mutable"
  "private"
  "rec"
  "type"
  "and"
  "assert"
  "await"
  "with"
  "lazy"
  "constraint"
] @keyword

((function "async" @keyword.storage))

(module_unpack "unpack" @keyword)

[
  "if"
  "else"
  "switch"
  "when"
] @keyword.control.conditional

[
  "exception"
  "try"
  "catch"
] @keyword.control.exception

(call_expression
  function: (value_identifier) @keyword.control.exception
  (#eq? @keyword.control.exception "raise"))

[
  "for"
  "in"
  "to"
  "downto"
  "while"
] @keyword.control.conditional

(ternary_expression ["?" ":"] @operator)

; JSX
;----------
(jsx_identifier) @tag
(jsx_element
  open_tag: (jsx_opening_element ["<" ">"] @punctuation.special))
(jsx_element
  close_tag: (jsx_closing_element ["<" "/" ">"] @punctuation.special))
(jsx_self_closing_element ["/" ">" "<"] @punctuation.special)
(jsx_fragment [">" "<" "/"] @punctuation.special)
(jsx_attribute (property_identifier) @attribute)
