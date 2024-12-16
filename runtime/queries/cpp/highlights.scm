; Functions

; These casts are parsed as function calls, but are not.
((identifier) @keyword (#eq? @keyword "static_cast"))
((identifier) @keyword (#eq? @keyword "dynamic_cast"))
((identifier) @keyword (#eq? @keyword "reinterpret_cast"))
((identifier) @keyword (#eq? @keyword "const_cast"))

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
(namespace_definition name: (namespace_identifier) @namespace)
(namespace_identifier) @namespace

(qualified_identifier name: (identifier) @type.enum.variant)

(auto) @type
"decltype" @type

(ref_qualifier ["&" "&&"] @type.builtin)
(reference_declarator ["&" "&&"] @type.builtin)
(abstract_reference_declarator ["&" "&&"] @type.builtin)

; Constants

(this) @variable.builtin
(nullptr) @constant.builtin

; Parameters

(parameter_declaration
  declarator: (reference_declarator (identifier) @variable.parameter))
(optional_parameter_declaration
  declarator: (identifier) @variable.parameter)

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
  "template"
] @keyword.storage.type

[
  "constexpr"
  "constinit"
  "consteval"
  "mutable"
] @keyword.storage.modifier

; Modifiers that aren't plausibly type/storage related.
[
  "explicit"
  "friend"
  "virtual"
  (virtual_specifier) ; override/final
  "private"
  "protected"
  "public"
  "inline" ; C++ meaning differs from C!
] @keyword

; Strings

(raw_string_literal) @string

; inherits: c
