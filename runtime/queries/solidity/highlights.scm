; identifiers
; -----------
(identifier) @variable
(yul_identifier) @variable

; Pragma
(pragma_directive) @tag
(solidity_version_comparison_operator _ @tag)


; Literals
; --------

[
 (string)
 (hex_string_literal)
 (unicode_string_literal)
 (yul_string_literal)
] @string
[
 (number_literal)
 (yul_decimal_number)
 (yul_hex_number)
] @constant.numeric
[
 (true)
 (false)
] @constant.builtin

(comment) @comment


; Definitions and references
; -----------

(type_name) @type
(primitive_type) @type
(user_defined_type (identifier) @type)

; Color payable in payable address conversion as type and not as keyword
(payable_conversion_expression "payable" @type)
; Ensures that delimiters in mapping( ... => .. ) are not colored like types
(type_name "(" @punctuation.bracket "=>" @punctuation.delimiter ")" @punctuation.bracket)

; Definitions
(struct_declaration 
  name: (identifier) @type)
(enum_declaration 
  name: (identifier) @type)
(contract_declaration
  name: (identifier) @type) 
(library_declaration
  name: (identifier) @type) 
(interface_declaration
  name: (identifier) @type)
(event_definition 
  name: (identifier) @type) 

(function_definition
  name:  (identifier) @function)

(modifier_definition
  name:  (identifier) @function)
(yul_evm_builtin) @function.builtin

; Use constructor coloring for special functions
(constructor_definition "constructor" @constructor)
(fallback_receive_definition "receive" @constructor)
(fallback_receive_definition "fallback" @constructor)

(struct_member name: (identifier) @variable.other.member)
(enum_value) @constant

; Invocations
(emit_statement . (identifier) @type)
(modifier_invocation (identifier) @function)

(call_expression . (member_expression property: (identifier) @function.method))
(call_expression . (identifier) @function)

; Function parameters
(call_struct_argument name: (identifier) @field)
(event_paramater name: (identifier) @variable.parameter)
(parameter name: (identifier) @variable.parameter)

; Yul functions
(yul_function_call function: (yul_identifier) @function)
(yul_function_definition . (yul_identifier) @function (yul_identifier) @variable.parameter)


; Structs and members
(member_expression property: (identifier) @variable.other.member)
(struct_expression type: ((identifier) @type .))
(struct_field_assignment name: (identifier) @variable.other.member)

; Tokens
; -------

; Keywords
(meta_type_expression "type" @keyword)
[
 "pragma"
 "contract"
 "interface"
 "library"
 "is"
 "struct"
 "enum"
 "event"
 "using"
 "assembly"
 "emit"
 "public"
 "internal"
 "private"
 "external"
 "pure"
 "view"
 "payable"
 "modifier"
 "memory"
 "storage"
 "calldata"
 "var"
 "constant"
 (virtual)
 (override_specifier)
 (yul_leave)
] @keyword

[
 "for"
 "while"
 "do"
] @keyword.control.repeat

[
 "break"
 "continue"
 "if"
 "else"
 "switch"
 "case"
 "default"
] @keyword.control.conditional

[
 "try"
 "catch"
] @keyword.control.exception

[
 "return"
 "returns"
] @keyword.control.return

"function" @keyword.function

"import" @keyword.control.import
(import_directive "as" @keyword.control.import)
(import_directive "from" @keyword.control.import)
(event_paramater "indexed" @keyword) ; TODO fix spelling once fixed upstream

; Punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket


[
  "."
  ","
] @punctuation.delimiter


; Operators

[
  "&&"
  "||"
  ">>"
  ">>>"
  "<<"
  "&"
  "^"
  "|"
  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "<"
  "<="
  "=="
  "!="
  "!=="
  ">="
  ">"
  "!"
  "~"
  "-"
  "+"
  "delete"
  "new"
  "++"
  "--"
] @operator

[
  "delete"
  "new"
] @keyword.operator
