; Attributes
; module declaration
(attribute
  name: (atom) @keyword
  (arguments (atom) @module)
 (#eq? @keyword "module"))

(attribute
  name: (atom) @keyword
  (arguments
    .
    (atom) @module)
 (#eq? @keyword "import"))

(attribute
  name: (atom) @keyword
  (arguments
    .
    (atom) @type
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
      (atom) @keyword.directive
      (variable) @keyword.directive
      (call
        function:
          [(variable) (atom)] @keyword.directive)
    ])
 (#eq? @keyword "define"))

(attribute
  name: (atom) @keyword
  module: (atom) @module
 (#eq? @keyword "spec"))

; Functions
(function name: (atom) @function)
(call module: (atom) @module)
(call function: (atom) @function)
(stab_clause name: (atom) @function)
(function_capture module: (atom) @module)
(function_capture function: (atom) @function)

; Records
(record_content
  (binary_operator
    left: (atom) @variable.other.member
    operator: "="))

(record field: (atom) @variable.other.member)
(record name: (atom) @type)

; Keywords
((attribute name: (atom) @keyword)
 (#match?
  @keyword
  "^(define|export|export_type|include|include_lib|ifdef|ifndef|if|elif|else|endif|vsn|on_load|behaviour|record|file|type|opaque|spec)$"))

["case" "fun" "if" "of" "when" "end" "receive" "try" "catch" "after" "maybe" "begin"] @keyword

; Operators
(binary_operator
  left: (atom) @function
  operator: "/"
  right: (integer) @constant.numeric.integer)

(binary_operator operator: _ @operator)
(unary_operator operator: _ @operator)
["/" ":" "#" "->"] @operator

; Comments
((variable) @comment.discard
 (#match? @comment.discard "^_"))

(tripledot) @comment.discard

(comment) @comment

; Macros
(macro
  "?"+ @keyword.directive
  name: (_) @keyword.directive)

; Basic types
(variable) @variable
[
  (atom)
  (quoted_atom)
] @string.special.symbol
(string) @string
(character) @constant.character

(integer) @constant.numeric.integer
(float) @constant.numeric.float

; Punctuation
["," "." "-" ";"] @punctuation.delimiter
["(" ")" "{" "}" "[" "]" "<<" ">>"] @punctuation.bracket

; (ERROR) @error
