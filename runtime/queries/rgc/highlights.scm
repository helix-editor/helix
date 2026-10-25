; RGC syntax highlighting for Helix.
; Node names and field names follow the tree-sitter-rgc grammar.

(comment) @comment

(string) @string
(escape_sequence) @string.escape

(number) @number
(boolean) @boolean
(none) @constant.builtin

[
  "if" "elif" "else" "for" "in" "while" "match" "case"
  "def" "class" "enum" "interface" "import"
  "and" "or" "not" "return"
] @keyword

(break_statement) @keyword
(continue_statement) @keyword
(pass_statement) @keyword

(type) @type.builtin

(function_definition
  name: (identifier) @function)

(call_expression
  function: (identifier) @function)

(class_definition
  name: (identifier) @type)

(enum_definition
  name: (identifier) @type)

(interface_definition
  name: (identifier) @type)

(member_expression
  property: (identifier) @property)

(field_declaration
  name: (identifier) @property)

(binary_expression
  operator: _ @operator)

(assignment
  operator: _ @operator)

(unary_expression
  ["!" "-" "not"] @operator)

(borrow_type
  "@" @attribute
  "var" @attribute)

(borrow_expression
  "@" @attribute
  "var" @attribute)

["," "." ":" ";"] @punctuation.delimiter
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
