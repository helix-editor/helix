; Tokens
;-------

[
  ";"
  (optional_chain) ; ?.
  "."
  ","
] @punctuation.delimiter

[
  "-"
  "--"
  "-="
  "+"
  "++"
  "+="
  "*"
  "*="
  "**"
  "**="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "~"
  "^"
  "&"
  "|"
  "^="
  "&="
  "|="
  "&&"
  "||"
  "??"
  "&&="
  "||="
  "??="
  "..."
] @operator

(ternary_expression ["?" ":"] @operator)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

[
  "async"
  "debugger"
  "extends"
  "from"
  "get"
  "new"
  "set"
  "target"
  "with"
] @keyword

[
  "of"
  "as"
  "in"
  "delete"
  "typeof"
  "instanceof"
  "void"
] @keyword.operator

[
  "function"
] @keyword.function

[
  "class"
  "let"
  "var"
] @keyword.storage.type

[
  "const"
  "static"
] @keyword.storage.modifier

[
  "default"
  "yield"
  "finally"
  "do"
  "await"
] @keyword.control

[
  "if"
  "else"
  "switch"
  "case"
] @keyword.control.conditional

[
  "while"
  "for"
] @keyword.control.repeat

[
  "import"
  "export"
] @keyword.control.import

[
  "return"
  "break"
  "continue"
] @keyword.control.return

[
  "throw"
  "try"
  "catch"
] @keyword.control.exception

; Variables
;----------

(identifier) @variable

; Properties
;-----------

(property_identifier) @variable.other.member
(private_property_identifier) @variable.other.member.private
(shorthand_property_identifier) @variable.other.member
(shorthand_property_identifier_pattern) @variable.other.member

; Function and method definitions
;--------------------------------

(function_expression
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)
(method_definition
  name: (property_identifier) @function.method)
(method_definition
  name: (private_property_identifier) @function.method.private)

(pair
  key: (property_identifier) @function.method
  value: [(function_expression) (arrow_function)])
(pair
  key: (private_property_identifier) @function.method.private
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.method)
  right: [(function_expression) (arrow_function)])
(assignment_expression
  left: (member_expression
    property: (private_property_identifier) @function.method.private)
  right: [(function_expression) (arrow_function)])

(variable_declarator
  name: (identifier) @function
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (identifier) @function
  right: [(function_expression) (arrow_function)])

; Function and method parameters
;-------------------------------

; Arrow function parameters in the form `p => ...` are supported by both
; javascript and typescript grammars without conflicts.
(arrow_function
  parameter: (identifier) @variable.parameter)

; Function and method calls
;--------------------------

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: (property_identifier) @function.method))
(call_expression
  function: (member_expression
    property: (private_property_identifier) @function.method.private))

; Literals
;---------

(this) @variable.builtin
(super) @variable.builtin

[
  (null)
  (undefined)
] @constant.builtin

[
  (true)
  (false)
] @constant.builtin.boolean

(comment) @comment

[
  (string)
  (template_string)
] @string

(escape_sequence) @constant.character.escape

(regex) @string.regexp

; future-proof fall-back, and `TypedArray` values (looks like float, but is int)
(number) @constant.numeric
; https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#prod-NumericLiteral
((number) @constant.numeric.float
  (#match? @constant.numeric.float "[^n]$"))
((number) @constant.numeric.integer
  (#match? @constant.numeric.integer "^[^.][^.\-]*n$"))


; Special identifiers
;--------------------

((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

([
    (identifier)
    (shorthand_property_identifier)
    (shorthand_property_identifier_pattern)
 ] @constant
 (#match? @constant "^[A-Z_][A-Z\\d_]+$"))

; Built-in constructors / types
((identifier) @type.builtin
 (#any-of? @type.builtin
  "Object" "Function" "Boolean" "Symbol" "Number" "BigInt" "String" "RegExp"
  "Array" "Map" "Set" "WeakMap" "WeakSet" "WeakRef" "Promise" "Proxy" "Date"
  "ArrayBuffer" "SharedArrayBuffer" "DataView"
  "Int8Array" "Uint8Array" "Uint8ClampedArray" "Int16Array" "Uint16Array"
  "Int32Array" "Uint32Array" "Float32Array" "Float64Array"
  "BigInt64Array" "BigUint64Array"
  "Error" "EvalError" "RangeError" "ReferenceError" "SyntaxError" "TypeError"
  "URIError" "AggregateError")
 (#is-not? local))

((identifier) @variable.builtin
 (#match? @variable.builtin "^(arguments|module|console|window|document|globalThis|Math|JSON|Reflect|Intl)$")
 (#is-not? local))

(call_expression
 (identifier) @function.builtin
 (#any-of? @function.builtin
  "eval"
  "fetch"
  "isFinite"
  "isNaN"
  "parseFloat"
  "parseInt"
  "decodeURI"
  "decodeURIComponent"
  "encodeURI"
  "encodeURIComponent"
  "require"
  "alert"
  "prompt"
  "btoa"
  "atob"
  "confirm"
  "structuredClone"
  "setTimeout"
  "clearTimeout"
  "setInterval"
  "clearInterval"
  "queueMicrotask")
 (#is-not? local))

; Shebang
;--------

(hash_bang_line) @keyword.directive

; Labels
;-------

(statement_identifier) @label

; Decorators
;-----------

(decorator "@" @attribute)
(decorator (identifier) @attribute)
(decorator
  (call_expression
    function: (identifier) @attribute))
(decorator
  (member_expression
    property: (property_identifier) @attribute))
(decorator
  (call_expression
    function: (member_expression
      property: (property_identifier) @attribute)))

; Namespace imports/exports: `import * as ns` / `export * as ns`
;----------------------------------------------------------------

(namespace_import (identifier) @namespace)
(namespace_export (identifier) @namespace)
