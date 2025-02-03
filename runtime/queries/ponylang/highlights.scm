[
  (line_comment)
  (block_comment)
] @comment

(bool) @constant.builtin.boolean
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(character) @constant.character

;; strings and docstring
(string) @string
(source_file docstring: (string) @string.special)
(entity docstring: (string) @string.special)
(method docstring: (string) @string.special) ; docstring for methods without body
(behavior docstring: (string) @string.special) ; docstring for methods without body
(constructor docstring: (string) @string.special) ; docstring for methods without body
(method body: (block . (string) @string.special)) ; docstring for methods with body
(behavior body: (block . (string) @string.special))
(constructor body: (block . (string) @string.special))
(field docstring: (string) @string.special)

;; Punctuation
[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket
[
  ";"
  "."
  ","
] @punctuation.delimiter

(this) @variable.builtin

(field name: (identifier) @variable.other.member)

"use" @keyword.control.import
[
  "for"
  "in"
  "while"
  "do"
  "repeat"
  "until"
] @keyword.control.repeat
[
 "if"
 "ifdef"
 "iftype"
 "then"
 "elseif"
 "else"
 "match"
] @keyword.control.conditional
[
  "break"
  "continue"
  "return"
  "error"
  "compile_error"
  "compile_intrinsic"
] @keyword.control.return
[
  "recover"
  "consume"
  "end"
  "try"
  "with"
] @keyword.control

[
  "as"
  "is"
  "isnt"
  "not"
  "and"
  "or"
  "xor"
  "digestof"
  "addressof"
  (location)
] @keyword.operator

(entity_type) @keyword.storage.type

[
  "var"
  "let"
  "embed"
] @keyword.storage

[
  "fun"
  "be"
  "new"
] @keyword.function

[
  (cap)
  (gencap)
  "where"
] @keyword

[
  (partial)
  "=>"
  "~"
  ".>"
  "+"
  "-"
  "*"
  "/"
  "%"
  "%%"
  "+~"
  "-~"
  "/~"
  "*~"
  "%~"
  "%%~"

  ">>"
  "<<"
  ">>~"
  "<<~"

  "=="
  "!="
  ">"
  "<"
  ">="
  "<="
] @operator

;; variables
;; references to upper case things are considered constructors
(identifier) @variable
(
  (identifier) @constructor
  (#match? @constructor "^[A-Z]")
)

;; Types
(entity name: (identifier) @type)
(nominal_type name: (identifier) @type)
(typeparams (typeparam name: (identifier) @type))

;; constructors / methods / behaviors
(constructor name: (identifier) @constructor)
(method name: (identifier) @function.method)
(behavior name: (identifier) @function.method)

;; method calls
; TODO: be more specific about what is the actual function reference
(call callee: (field_access field: (identifier) @function.method))
(call callee: (_) @function.method)
(ffi_call name: (_) @function)
(partial_application function: (identifier) @function.method)
(chain function: (identifier) @function.method)

;; fields and params
(field name: (identifier) @variable.other.member)
(param (identifier) @variable.parameter)
(lambdaparam (identifier) @variable.parameter)

;; this.field is considered a member access
(field_access base: (this) field: (identifier) @variable.other.member)

;; annotations
(annotations (identifier) @attribute)

