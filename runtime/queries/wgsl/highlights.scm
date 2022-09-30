(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(bool_literal) @constant.builtin.boolean

(global_constant_declaration) @variable
(global_variable_declaration) @variable
(compound_statement) @variable
(const_expression) @function

(variable_identifier_declaration
    (identifier) @variable
    (type_declaration) @type)

(function_declaration
    (identifier) @function
    (function_return_type_declaration
        (type_declaration) @type))

(parameter
    (variable_identifier_declaration
        (identifier) @variable.parameter
        (type_declaration) @type))

(struct_declaration
    (identifier) @type)
        
(struct_declaration
    (struct_member
        (variable_identifier_declaration
            (identifier) @variable.other.member
            (type_declaration) @type)))

(type_constructor_or_function_call_expression
    (type_declaration) @function)

[
    "struct"
    "bitcast"
    "discard"
    "enable"
    "fallthrough"
    "fn"
    "let"
    "private"
    "read"
    "read_write"
    "storage"
    "type"
    "uniform"
    "var"
    "workgroup"
    "write"
    "override"
    (texel_format)
] @keyword

"fn" @keyword.function

"return" @keyword.control.return

["," "." ":" ";"] @punctuation.delimiter

["(" ")" "[" "]" "{" "}"] @punctuation.bracket

[
    "loop"
    "for"
    "while"
    "break"
    "continue"
    "continuing"
] @keyword.control.repeat

[
    "if"
    "else"
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
    "@"
    "++"
    "--"
] @operator

(attribute
    (identifier) @attribute)

(comment) @comment

(ERROR) @error
