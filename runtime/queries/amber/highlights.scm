(comment) @comment

[
    "if"
    "loop"
    "for"
    "return"
    "fun"
    "else"
    "then"
    "break"
    "continue"
    "and"
    "or"
    "not"
    "let"
    "pub"
    "main"
    "echo"
    "exit"
    "fun"
    "import"
    "from"
    "as"
    "in"
    "fail"
    "failed"
    "silent"
    "nameof"
    "is"
    "unsafe"
    "trust"
] @keyword

; Literals
(boolean) @constant.builtin.boolean
(number) @constant.numeric
(null) @constant.numeric
(string) @string
(status) @keyword
(command) @string
(handler) @keyword
(block) @punctuation.delimiter
(variable_init) @keyword
(variable_assignment) @punctuation.delimiter
(variable) @variable
(escape_sequence) @constant.character.escape
(type_name_symbol) @type
(interpolation) @punctuation.delimiter
(reference) @keyword
(preprocessor_directive) @comment
(shebang) @comment
(function_definition
    name: (variable) @function.method)
(function_call
    name: (variable) @function.method)
(import_statement
    "pub" @keyword
    "import" @keyword
    "from" @keyword)
