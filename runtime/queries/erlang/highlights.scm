; Attributes
; module declaration
(attribute
  name: (atom) @keyword
  (arguments (atom) @namespace)
 (#match? @keyword "(module|behaviou?r)"))

(attribute
  name: (atom) @keyword
  (arguments
    .
    (atom) @namespace)
 (#eq? @keyword "import"))

(attribute
  name: (atom) @keyword
  (arguments
    .
    [(atom) @type (macro)]
    [
      (tuple (atom) @variable.other.member)
      (tuple
        (binary_operator
          left: (atom) @variable.other.member
          operator: ["=" "::"]))
      (tuple
        (binary_operator
          left:
            (binary_operator
              left: (atom) @variable.other.member
              operator: "=")
          operator: "::"))
      ])
 (#eq? @keyword "record"))

(attribute
  name: (atom) @keyword
  (arguments
    .
    [
      (atom) @constant
      (variable) @constant
      (call
        function:
          [(variable) (atom)] @keyword.directive)
    ])
 (#eq? @keyword "define"))

(attribute
  name: (atom) @keyword
  (arguments
    (_) @keyword.directive)
 (#match? @keyword "ifn?def"))

(attribute
  name: (atom) @keyword
  module: (atom) @namespace
 (#match? @keyword "(spec|callback)"))

; Functions
(function_clause name: (atom) @function)
(call module: (atom) @namespace)
(call function: (atom) @function)
(stab_clause name: (atom) @function)
(function_capture module: (atom) @namespace)
(function_capture function: (atom) @function)

; Records
(record_content
  (binary_operator
    left: (atom) @variable.other.member
    operator: "="))

(record field: (atom) @variable.other.member)
(record name: (atom) @type)

; Keywords
(attribute name: (atom) @keyword)

["case" "fun" "if" "of" "when" "end" "receive" "try" "catch" "after" "begin" "maybe"] @keyword

; Operators
(binary_operator
  left: (atom) @function
  operator: "/"
  right: (integer) @constant.numeric.integer)

((binary_operator operator: _ @keyword.operator)
 (#match? @keyword.operator "^\\w+$"))
((unary_operator operator: _ @keyword.operator)
 (#match? @keyword.operator "^\\w+$"))

(binary_operator operator: _ @operator)
(unary_operator operator: _ @operator)
["/" ":" "->"] @operator

(tripledot) @comment.discard

(comment) @comment

; Macros
(macro
  "?"+ @constant
  name: (_) @constant
  !arguments)

(macro
  "?"+ @keyword.directive
  name: (_) @keyword.directive)

; Comments
((variable) @comment.discard
 (#match? @comment.discard "^_"))

; Basic types
(variable) @variable
(atom) @string.special.symbol
(string) @string
(character) @constant.character

(integer) @constant.numeric.integer
(float) @constant.numeric.float

; Punctuation
["," "." "-" ";"] @punctuation.delimiter
["(" ")" "#" "{" "}" "[" "]" "<<" ">>"] @punctuation.bracket

; (ERROR) @error
