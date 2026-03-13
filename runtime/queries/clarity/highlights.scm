; Comments
(comment) @comment

; Literals
[
  (int_lit)
  (uint_lit)
] @constant.numeric.integer

(bool_lit) @constant.builtin.boolean
(none_lit) @constant.builtin

[
  (ascii_string_lit)
  (utf8_string_lit)
] @string

[
  (buffer_lit)
  (standard_principal_lit)
  (contract_principal_lit)
] @string.special

; Types
[
  (native_type)
  (trait_type)
] @type

; Punctuation
[
  "("
  ")"
  "{"
  "}"
] @punctuation.bracket

(trait_type "<" @punctuation.bracket)
(trait_type ">" @punctuation.bracket)

[
  ","
] @punctuation.delimiter

; Keywords
(list_lit_token) @keyword
(some_lit ("some") @keyword)
(response_lit [
  "ok"
  "err"
] @keyword)

(int_lit "-" @constant.numeric.integer)

; Functions
(function_signature (identifier) @function)
(function_signature_for_trait (identifier) @function)
(contract_function_call operator: (identifier) @function)

(basic_native_form operator: (native_identifier) @function.builtin)
(basic_native_form operator: (native_identifier) @keyword.operator
  (#match? @keyword.operator "^([+\\-*/<>]|<=|>=|mod|pow|and|or|xor|not)$"))
[
  "let"
] @function.builtin

[
  "impl-trait"
  "use-trait"
] @keyword.control.import

[
  "define-read-only"
  "define-private"
  "define-public"
] @keyword.function

[
  "define-trait"
  "define-constant"
  "define-data-var"
  "define-map"
  "define-fungible-token"
  "define-non-fungible-token"
] @keyword.storage.type

; Variables and parameters
(function_parameter) @variable.parameter
(trait_usage trait_alias: (identifier) @type.parameter)

(tuple_lit key: (identifier) @variable.other.member)
(tuple_type key: (identifier) @variable.other.member)
(tuple_type_for_trait key: (identifier) @variable.other.member)

(global) @variable.builtin
