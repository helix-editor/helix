(const_literal) @constant.numeric

(type_declaration) @type

(function_declaration
    (identifier) @function)

(struct_declaration
    (identifier) @type)

(type_constructor_or_function_call_expression
    (type_declaration) @function)

(parameter
    (variable_identifier_declaration (identifier) @variable.parameter))

[
    "struct"
    "bitcast"
    ; "block"
    "discard"
    "enable"
    "fallthrough"
    "fn"
    "let"
    "private"
    "read"
    "read_write"
    "return"
    "storage"
    "type"
    "uniform"
    "var"
    "workgroup"
    "write"
    (texel_format)
] @keyword ; TODO reserved keywords

[
    (true)
    (false)
] @constant.builtin.boolean

[ "," "." ":" ";" ] @punctuation.delimiter

;; brackets
[
    "("
    ")"
    "["
    "]"
    "{"
    "}"
] @punctuation.bracket

[
    "loop"
    "for"
    "break"
    "continue"
    "continuing"
] @keyword.control.repeat

[
    "if"
    "else"
    "elseif"
    "switch"
    "case"
    "default"
] @keyword.control.conditional

[
    "&"
    "&&"
    "/"
    "!"
    "="
    "=="
    "!="
    ">"
    ">="
    ">>"
    "<"
    "<="
    "<<"
    "%"
    "-"
    "+"
    "|"
    "||"
    "*"
    "~"
    "^"
] @operator

(attribute
    (identifier) @variable.other.member)

(comment) @comment

(ERROR) @error
