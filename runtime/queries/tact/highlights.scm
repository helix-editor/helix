; See: https://docs.helix-editor.com/master/themes.html#syntax-highlighting
; -------------------------------------------------------------------------

; attribute
; ---------

[
  "@name"
  "@interface"
] @attribute

; operator
; --------

[
  "-" "-="
  "+" "+="
  "*" "*="
  "/" "/="
  "%" "%="
  "=" "=="
  "!" "!=" "!!"
  "<" "<=" "<<"
  ">" ">=" ">>"
  "&" "|"
  "&&" "||"
] @operator

; punctuation.bracket
; -------------------

[
  "(" ")"
  "{" "}"
] @punctuation.bracket

; punctuation.delimiter
; ---------------------

[
  ";"
  ","
  "."
  ":"
  "?"
] @punctuation.delimiter

; variable
; --------

(identifier) @variable

; variable.builtin
; ----------------

(self) @variable.builtin

; variable.parameter
; ------------------

(parameter
  name: (identifier) @variable.parameter)

; variable.other.member
; ---------------------

(field
  name: (identifier) @variable.other.member)

(contract_body
  (constant
    name: (identifier) @variable.other.member))

(trait_body
  (constant
    name: (identifier) @variable.other.member))

(field_access_expression
  name: (identifier) @variable.other.member)

(lvalue (_) (_) @variable.other.member)

(instance_argument
  name: (identifier) @variable.other.member)

; comment.block
; -------------

(comment) @comment.block

; comment.line
; ------------

((comment) @comment.line
  (#match? @comment.line "^//"))

; function
; --------

(func_identifier) @function

(native_function
  name: (identifier) @function)

(static_function
  name: (identifier) @function)

(static_call_expression
  name: (identifier) @function)

(init_function
  "init" @function.method)

(receive_function
  "receive" @function.method)

(bounced_function
  "bounced" @function.method)

(external_function
  "external" @function.method)

(function
  name: (identifier) @function.method)

; function.method
; ---------------

(method_call_expression
  name: (identifier) @function.method)

; function.builtin
; ----------------

((identifier) @function.builtin
  (#any-of? @function.builtin
    "send" "sender" "require" "now"
    "myBalance" "myAddress" "newAddress"
    "contractAddress" "contractAddressExt"
    "emit" "cell" "ton"
    "beginString" "beginComment" "beginTailString" "beginStringFromBuilder" "beginCell" "emptyCell"
    "randomInt" "random"
    "checkSignature" "checkDataSignature" "sha256"
    "min" "max" "abs" "pow"
    "throw" "dump" "getConfigParam"
    "nativeThrowWhen" "nativeThrowUnless" "nativeReserve"
    "nativeRandomize" "nativeRandomizeLt" "nativePrepareRandom" "nativeRandom" "nativeRandomInterval")
  (#is-not? local))

; keyword.control.conditional
; ---------------------------

[
  "if" "else"
] @keyword.control.conditional

; keyword.control.repeat
; ----------------------

[
  "while" "repeat" "do" "until"
] @keyword.control.repeat

; keyword.control.import
; ----------------------

"import" @keyword.control.import

; keyword.control.return
; ----------------------

"return" @keyword.control.return

; keyword.operator
; ----------------

"initOf" @keyword.operator

; keyword.directive
; -----------------

"primitive" @keyword.directive

; keyword.function
; ----------------

[
  "fun"
  "native"
] @keyword.function

; keyword.storage.type
; --------------------

[
  "contract" "trait" "struct" "message" "with"
  "const" "let"
] @keyword.storage.type

; keyword.storage.modifier
; ------------------------

[
  "get" "mutates" "extends" "virtual" "override" "inline" "abstract"
] @keyword.storage.modifier

; keyword
; -------

[
  "with"
  ; "public" ; -- not used, but declared in grammar.ohm
  ; "extend" ; -- not used, but declared in grammar.ohm
] @keyword

; constant.builtin.boolean
; ------------------------

(boolean) @constant.builtin.boolean

; constant.builtin
; ----------------

((identifier) @constant.builtin
  (#any-of? @constant.builtin
    "SendPayGasSeparately"
    "SendIgnoreErrors"
    "SendDestroyIfZero"
    "SendRemainingValue"
    "SendRemainingBalance")
  (#is-not? local))

(null) @constant.builtin

; constant.numeric.integer
; ------------------------

(integer) @constant.numeric.integer

; constant
; --------

(constant
  name: (identifier) @constant)

; string
; ------

(string) @string

; string.special.path
; -------------------

(import_statement
  library: (string) @string.special.path)

; type
; ----

(type_identifier) @type

; type.builtin
; ------------

(tlb_serialization
  "as" @keyword
  type: (identifier) @type.builtin
  (#any-of? @type.builtin
    "int8" "int16" "int32" "int64" "int128" "int256" "int257"
    "uint8" "uint16" "uint32" "uint64" "uint128" "uint256"
    "coins" "remaining" "bytes32" "bytes64"))

((type_identifier) @type.builtin
  (#any-of? @type.builtin
    "Address" "Bool" "Builder" "Cell" "Int" "Slice" "String" "StringBuilder"))

(map_type
  "map" @type.builtin
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(bounced_type
  "bounced" @type.builtin
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

((identifier) @type.builtin
  (#eq? @type.builtin "SendParameters")
  (#is-not? local))

; constructor
; -----------

(instance_expression
  name: (identifier) @constructor)

(initOf
  name: (identifier) @constructor)
