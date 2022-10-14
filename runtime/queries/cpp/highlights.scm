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

(using_declaration ("using" "namespace" (identifier) @namespace))
(using_declaration ("using" "namespace" (qualified_identifier name: (identifier) @namespace)))
(namespace_definition name: (identifier) @namespace)
(namespace_identifier) @namespace

(qualified_identifier name: (identifier) @type.enum.variant)

(auto) @type
"decltype" @type

; Constants

(this) @variable.builtin
(nullptr) @constant.builtin

; Keywords

(template_argument_list (["<" ">"] @punctuation.bracket))
(template_parameter_list (["<" ">"] @punctuation.bracket))
(default_method_clause "default" @keyword)

"static_assert" @function.special

[
  "<=>"
  "[]"
  "()"
] @operator

[
  "co_await"
  "co_return"
  "co_yield"
  "concept"
  "delete"
  "final"
  "new"
  "operator"
  "requires"
  "using"
] @keyword

[
  "catch"
  "noexcept"
  "throw"
  "try"
] @keyword.control.exception


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
  "typename"
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
