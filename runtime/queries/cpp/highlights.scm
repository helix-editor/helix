; Functions

(call_expression
  function: (qualified_identifier
    name: (identifier) @function))

(template_function
  name: (identifier) @function)

(template_method
  name: (field_identifier) @function)

(function_declarator
  declarator: (qualified_identifier
    name: (identifier) @function))

(function_declarator
  declarator: (qualified_identifier
    name: (qualified_identifier
      name: (identifier) @function)))

(function_declarator
  declarator: (field_identifier) @function)

; Types

(namespace_identifier) @namespace
(auto) @type

; Constants

(this) @variable.builtin
(nullptr) @constant.builtin

; Keywords

[
  "catch"
  "co_await"
  "co_return"
  "co_yield"
  "concept"
  "consteval"
  "constinit"
  "delete"
  "final"
  "noexcept"
  "new"
  "requires"
  "throw"
  "try"
  "typename"
  "using"
] @keyword

"<=>" @operator

[
  "or"
  "and"
  "bitor"
  "xor"
  "bitand"
  "not_eq"
  "and_eq"
  "or_eq"
  "xor_eq"
] @keyword.operator

[
  "class"
  "namespace"
] @keyword.storage.type

[
  "constexpr"
  "constinit"
  "consteval"
  "explicit"
  "friend"
  "mutable"
  "private"
  "protected"
  "public"
  "override"
  "template"
  "virtual"
] @keyword.storage.modifier

; Strings

(raw_string_literal) @string

; inherits: c
