(type) @type
(nullable_type) @type

[
  "case"
  "match"
  "switch"
] @keyword.control.conditional

[
  "export"
  "use"
] @keyword.control.import

"fn" @keyword.function

[
  "const"
  "nullable"
] @keyword.storage.modifier

[
  "def"
  "defer"
  "enum"
  "static"
  "struct"
  "type"
  "union"
  "_"
] @keyword

[
  "%="
  "&&="
  "&="
  "*="
  "+="
  "-="
  "."
  ".."
  "/="
  "::"
  "<<="
  "="
  "=>"
  ">>="
  "?"
  "^="
  "^^="
  "|="
  "||="
] @operator

[
  "("
  ")"
  "["
  "]"
  ")"
  "{"
  "}"
] @punctuation.bracket

[
  ":"
  ";"
] @punctuation.delimiter

"..." @special

(comment) @comment

((literal) @constant.builtin
 (#match? @constant.builtin "^(null|void)$"))

((literal) @constant.builtin.boolean
 (#match? @constant.builtin.boolean "^(false|true)$"))

(integer_literal) @constant.numeric.integer
(floating_literal) @constant.numeric.float
(rune_literal) @constant.character
(escape_sequence) @constant.character.escape
(string_literal) @string

(call_expression
  (postfix_expression
    (nested_expression
      (identifier) @function)))

(allocation_expression
  _ @function.builtin
  (#match? @function.builtin "^(alloc|free)$"))

(align_expression "align" @function.builtin)
(size_expression "size" @function.builtin)
(length_expression "len" @function.builtin)
(offset_expression "offset" @function.builtin)

(append_expression
  _ @function.builtin
 (#match? @function.builtin "^(append|delete|insert)$"))

(assertion_expression
  _ @keyword.control.exception
  (#match? @keyword.control.exception "^(abort|assert)$"))

(unary_expression
  _ @operator
  (#match? @operator "^[+-~!*&]$"))

(multiplicative_expression
  _ @operator
  (#match? @operator "^[*/%]$"))

(additive_expression
  _ @operator
  (#match? @operator "^[+-]$"))

(shift_expression
  _ @operator
  (#match? @operator "^(<<|>>)$"))

(and_expression "&" @operator)
(exclusive_or_expression "^" @operator)
(inclusive_or_expression "|" @operator)

(comparison_expression
  _ @operator
  (#match? @operator "^(<|>|<=|>=)$"))

(equality_expression
  _ @operator
  (#match? @operator "^(==|!=)$"))

(logical_and_expression "&&" @operator)
(logical_xor_expression "^^" @operator)
(logical_or_expression "||" @operator)

(if_expression
  _ @keyword.control.conditional
  (#match? @keyword.control.conditional "^(else|if)$"))

(for_loop "for" @keyword.control.repeat)

(iterable_binding
  _ @keyword.storage.type
  (#match? @keyword.storage.type "^(const|let)$"))

(switch_expression "switch" @keyword.control.conditional)
(switch_case "case" @keyword.control.conditional)
(match_expression "match" @keyword.control.conditional)
(match_case "case" @keyword.control.conditional)
(match_case "let" @keyword.storage.type)

(control_expression
  _ @keyword.control.repeat
  (#match? @keyword.control.repeat "^(break|continue)$"))

(control_expression
  _ @keyword.control.return
  (#match? @keyword.control.repeat "^(return|yield)$"))

(cast_expression
  _ @keyword.operator
  (#match? @keyword.operator "^(as|is)$"))

(variadic_expression
  _ @function.builtin
  (#match? @function.builtin "$va(arg|end|start)$"))

(function_declaration
  name: (identifier) @function)

(parameter
  (name) @variable.parameter)

(field_access_expression
  selector: (name) @variable.other.member)

(identifier) @variable

(struct_union_field
  (name) @variable)

(decl_attr) @special
(fndec_attrs) @special

(ERROR) @error
