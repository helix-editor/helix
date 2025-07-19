(comment) @comment

; these are listed first, because they override keyword queries
(function_expression (identifier) @function)

[
    (assignment_operator)
    (additive_operator)
    (multiplicative_operator)
    (equality_operator)
    ">="
    "<="
    "<"
    ">"
    "+"
    "-"
] @operator

[
    (and)
    (or)
    (not)
    (in)
] @keyword.operator

[
    "(" ")" "[" "]" "{" "}"
] @punctuation.bracket

[
    (if)
    (elif)
    (else)
    (endif)
] @keyword.control.conditional

[
    (foreach)
    (endforeach)
    (break)
    (continue)
] @keyword.control.repeat

(boolean_literal) @constant.builtin.boolean
(int_literal) @constant.numeric.integer

(keyword_argument keyword: (identifier) @variable.parameter)
(escape_sequence) @constant.character.escape
(bad_escape) @warning

[
"."
","
":"
] @punctuation.delimiter

[
    (string_literal)
    (fstring_literal)
] @string

(identifier) @variable
