(comment) @comment

; Pragma
(pragma_directive) @tag
(solidity_version_comparison_operator ">=" @tag)
(solidity_version_comparison_operator "<=" @tag)
(solidity_version_comparison_operator "=" @tag)
(solidity_version_comparison_operator "~" @tag)
(solidity_version_comparison_operator "^" @tag)


; Literals
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


; Type
(type_name) @type
(primitive_type) @type
(struct_declaration struct_name: (identifier) @type)
(enum_declaration enum_type_name: (identifier) @type)
; Color payable in payable address conversion as type and not as keyword
(payable_conversion_expression "payable" @type)
(emit_statement . (identifier) @type)
; Handles ContractA, ContractB in function foo() override(ContractA, contractB) {}
(override_specifier (identifier) @type)
; Ensures that delimiters in mapping( ... => .. ) are not colored like types
(type_name "(" @punctuation.bracket "=>" @punctuation.delimiter ")" @punctuation.bracket)



; Functions and parameters

(function_definition
  function_name:  (identifier) @function)
(modifier_definition
  name:  (identifier) @function)
(yul_evm_builtin) @function.builtin

; Use constructor coloring for special functions
(constructor_definition "constructor" @constructor)
(fallback_receive_definition "receive" @constructor)
(fallback_receive_definition "fallback" @constructor)

(modifier_invocation (identifier) @function)

; Handles expressions like structVariable.g();
(call_expression . (member_expression (property_identifier) @function.method))

; Handles expressions like g();
(call_expression . (identifier) @function)

; Function parameters
(event_paramater name: (identifier) @variable.parameter) ; TODO fix spelling once fixed upstream
(function_definition
  function_name:  (identifier) @variable.parameter)

; Yul functions
(yul_function_call function: (yul_identifier) @function)

; Yul function parameters
(yul_function_definition . (yul_identifier) @function (yul_identifier) @variable.parameter)

(meta_type_expression "type" @keyword)

(member_expression (property_identifier) @variable.other.member)
(property_identifier) @variable.other.member
(struct_expression ((identifier) @variable.other.member . ":"))
(enum_value) @variable.other.member


; Keywords
[
 "pragma"
 "import"
 "contract"
 "interface"
 "library"
 "is"
 "struct"
 "enum"
 "event"
 "using"
 "assembly"
 "switch"
 "case"
 "default"
 "break"
 "continue"
 "if"
 "else"
 "for"
 "while"
 "do"
 "try"
 "catch"
 "return"
 "emit"
 "public"
 "internal"
 "private"
 "external"
 "pure"
 "view"
 "payable"
 "modifier"
 "returns"
 "memory"
 "storage"
 "calldata"
 "function"
 "var"
 (constant)
 (virtual)
 (override_specifier)
 (yul_leave)
] @keyword

(import_directive "as" @keyword)
(import_directive "from" @keyword)
(event_paramater "indexed" @keyword) ; TODO fix spelling once fixed upstream

; Punctuation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket


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

(identifier) @variable
(yul_identifier) @variable
