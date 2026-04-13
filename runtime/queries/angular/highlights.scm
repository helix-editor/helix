; inherits: html

(identifier) @variable

(pipe_operator) @operator

(number) @number

(pipe_call
  name: (identifier) @function)

(pipe_call
  arguments: (pipe_arguments
    (identifier) @variable.parameter))

(structural_directive
  "*" @keyword
  (identifier) @keyword)

(attribute
  (attribute_name) @variable.member
  (#match? @variable.member "^#"))

(binding_name
  (identifier) @keyword)

(event_binding
  (binding_name
    (identifier) @keyword))

(event_binding
  "\"" @punctuation.delimiter)

(property_binding
  "\"" @punctuation.delimiter)

(structural_assignment
  operator: (identifier) @keyword)

(member_expression
  property: (identifier) @property)

(call_expression
  function: (identifier) @function)

(call_expression
  function: ((identifier) @function.builtin
    (#eq? @function.builtin "$any")))

(pair
  key: ((identifier) @variable.builtin
    (#eq? @variable.builtin "$implicit")))

[
  (control_keyword)
  (special_keyword)
] @keyword

((control_keyword) @keyword.repeat
  (#any-of? @keyword.repeat "for" "empty"))

((control_keyword) @keyword.conditional
  (#any-of? @keyword.conditional "if" "else" "switch" "case" "default"))

((control_keyword) @keyword.coroutine
  (#any-of? @keyword.coroutine "defer" "placeholder" "loading"))

((control_keyword) @keyword.exception
  (#eq? @keyword.exception "error"))

((identifier) @boolean
  (#any-of? @boolean "true" "false"))

((identifier) @variable.builtin
  (#any-of? @variable.builtin "this" "$event"))

((identifier) @constant.builtin
  (#eq? @constant.builtin "null"))

[
  (ternary_operator)
  (conditional_operator)
] @keyword.conditional.ternary

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "@"
] @punctuation.bracket

(two_way_binding
  [
    "[("
    ")]"
  ] @punctuation.bracket)

[
  "{{"
  "}}"
] @punctuation.special

(template_substitution
  [
    "${"
    "}"
  ] @punctuation.special)

(template_chars) @string

[
  ";"
  "."
  ","
  "?."
] @punctuation.delimiter

(nullish_coalescing_expression
  (coalescing_operator) @operator)

(concatenation_expression
  "+" @operator)

(icu_clause) @keyword.operator

(icu_category) @keyword.conditional

(binary_expression
  [
    "-"
    "&&"
    "+"
    "<"
    "<="
    "="
    "=="
    "==="
    "!="
    "!=="
    ">"
    ">="
    "*"
    "/"
    "||"
    "%"
  ] @operator)

(tag_name) @tag
