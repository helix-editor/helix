;; Harneet Programming Language - Syntax Highlighting for Helix Editor
;; Tree-sitter highlighting queries

;; Keywords
[
  "var"
  "function" 
  "return"
  "defer"
  "if"
  "else"
  "for"
  "switch"
  "case"
  "default"
  "import"
  "as"
] @keyword

;; Control flow keywords
[
  "if"
  "else" 
  "for"
  "switch"
  "case"
  "default"
] @keyword.control

;; Function keywords
[
  "function"
  "return"
  "defer"
] @keyword.function

;; Import keywords
[
  "import"
  "as"
] @keyword.import

;; Storage keywords
[
  "var"
] @keyword.storage

;; Types
[
  "int"
  "int8"
  "int16" 
  "int32"
  "int64"
  "uint"
  "uint8"
  "uint16"
  "uint32" 
  "uint64"
  "uintptr"
  "float32"
  "float64"
  "string"
  "bool"
  "error"
] @type.builtin

;; Boolean literals
[
  "true"
  "false"
] @constant.builtin.boolean

;; Null/None
[
  "None"
] @constant.builtin

;; Operators
[
  "="
  ":="
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "and"
  "or"
  "not"
] @operator

;; Assignment operators
[
  "="
  ":="
] @operator.assignment

;; Logical operators
[
  "and"
  "or" 
  "not"
] @operator.logical

;; Punctuation
[
  ";"
  ":"
  ","
  "."
] @punctuation.delimiter

;; Brackets
[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

;; String literals
(string_literal) @string

;; Character literals  
(char_literal) @character

;; Number literals
(number_literal) @number
(float_literal) @number.float

;; Comments
(line_comment) @comment.line
(block_comment) @comment.block

;; Identifiers
(identifier) @variable

;; Function names
(function_declaration name: (identifier) @function)
(function_call function: (identifier) @function)

;; Module/package names
(module_identifier) @namespace

;; Constants (ALL_CAPS identifiers)
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_0-9]*$"))

;; Built-in functions and modules
((identifier) @function.builtin
 (#match? @function.builtin "^(fmt|math|strings|datetime|os|path|file|log|errors|json)$"))

;; Method calls
(method_call
  object: (identifier) @variable
  method: (identifier) @function.method)

;; Parameters
(parameter name: (identifier) @variable.parameter)

;; Error highlighting
(ERROR) @error