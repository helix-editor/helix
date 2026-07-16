; Keywords
"module" @keyword
"import" @keyword
"from" @keyword.control.import
"where" @keyword
"with" @keyword
"class" @keyword
"instance" @keyword
"let" @keyword
"in" @keyword
"case" @keyword.control.conditional
"of" @keyword
"if" @keyword.control.conditional
"then" @keyword.control.conditional
"else" @keyword.control.conditional
"implementation" @keyword
"definition" @keyword
"system" @keyword
"infix" @keyword
"infixl" @keyword
"infixr" @keyword
"generic" @keyword
"derive" @keyword

; Module names
(module_identifier) @namespace
(module_name (module_identifier) @namespace)

; Types
(type_signature name: (signature_name (identifier) @type))
(type_definition name: (constructor) @type)
(class_declaration name: (class_name (constructor) @type))
(class_name (constructor) @type)
(constructor) @type

; Functions
(function_declaration name: (identifier) @function)
(macro_definition name: (identifier) @function.macro)

; Variables and parameters
(identifier) @variable
(wildcard) @variable.builtin

; Literals
(string) @string
(char) @string.special
(number) @constant.numeric
(integer) @constant.numeric
(float) @constant.float
(boolean) @constant.builtin.boolean

; Comments
(line_comment) @comment.line
(block_comment) @comment.block

; Operators
(arrow) @operator
(comprehension_sep) @operator
(operator) @operator
(operator_add) @operator
(operator_mul) @operator
(operator_compare) @operator
(operator_cons) @operator
(operator_and) @operator
(operator_or) @operator
(operator_exp) @operator
(range_operator) @operator

; Patterns
(lazy_pattern "~" @operator)
(strict_pattern "!" @operator)

; Punctuation
"=" @operator
"," @punctuation.delimiter
"." @punctuation.delimiter
"|" @punctuation.delimiter
"::" @punctuation.delimiter
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
