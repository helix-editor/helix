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

(namespace_definition name: (identifier) @namespace)
(using_declaration (identifier) @namespace)
(namespace_identifier) @namespace

(qualified_identifier name: (identifier) @type.enum.variant)

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
  "and"
  "and_eq"
  "bitor"
  "bitand"
  "not"
  "not_eq"
  "or"
  "or_eq"
  "xor"
  "xor_eq"
] @keyword.operator

[
  "class"
  "namespace"
  (auto)
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
