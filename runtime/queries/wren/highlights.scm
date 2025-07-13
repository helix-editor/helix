(comment) @comment
(string) @string
(raw_string) @string
(number) @constant.numeric.integer
(name) @variable
(field) @variable
(static_field) @variable
(null) @constant.builtin
(boolean) @constant.builtin.boolean

((name) @variable.builtin
 (#match? @variable.builtin "^(Bool|Class|Fiber|Fn|List|Map|Null|Num|Object|Range|Sequence|String|System)$"))

(call_expression
  (name) @function)

(method_definition
  (name) @function.method)

((parameter) @variable.parameter)


(if_statement
[
  "if"
  "else"
] @keyword.control.conditional)

(for_statement
[
  "for"
  "in"
] @keyword.control.repeat)

(while_statement
[
  "while"
] @keyword.control.repeat)

[
  (break_statement)
  (continue_statement)
  (return_statement)
] @keyword.control.return

(class_definition
"is"
@keyword)

[
  "import"
  "for"
  "as"
] @keyword.control.import

[
  "is"
] @keyword

(operator) @operator

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

["," "."] @punctuation.delimiter

[
  "class"
  "var"
] @keyword.storage.type

[
  "static"
] @keyword.storage.modifier

(constructor
  ["construct"] @constructor)
