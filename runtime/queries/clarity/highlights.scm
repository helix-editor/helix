; Comments
(comment) @comment

; Literals
[
  (int_lit)
  (uint_lit)
] @constant.numeric.integer

[
  (bool_lit)
  (none_lit)
] @constant.builtin

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
  "<"
  ">"
] @punctuation.bracket

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

[
  "+"
  "-"
  "*"
  "/"
  "mod"
  "pow"
  "<"
  "<="
  ">"
  ">="
  "and"
  "or"
  "xor"
] @keyword.operator

; Functions
(function_signature (identifier) @function)
(function_signature_for_trait (identifier) @function)
(contract_function_call operator: (identifier) @function)

(basic_native_form operator: (native_identifier) @function.builtin)
[
  "let"
] @function.builtin

[
  "impl-trait"
  "use-trait"
  "define-trait"
  "define-read-only"
  "define-private"
  "define-public"
  "define-data-var"
  "define-fungible-token"
  "define-non-fungible-token"
  "define-constant"
  "define-map"
] @function.special

; Variables and parameters
(function_parameter) @variable.parameter
(trait_usage trait_alias: (identifier) @type.parameter)

(tuple_lit key: (identifier) @variable)
(tuple_type key: (identifier) @variable)
(tuple_type_for_trait key: (identifier) @variable)

(global) @variable.builtin

