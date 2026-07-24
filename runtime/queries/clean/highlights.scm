; Keywords
"module" @keyword
"import" @keyword
"from" @keyword
"where" @keyword
"with" @keyword
"class" @keyword
"instance" @keyword
"let" @keyword
"in" @keyword
"case" @keyword
"of" @keyword
"implementation" @keyword
"definition" @keyword
"system" @keyword
"infix" @keyword
"infixl" @keyword
"infixr" @keyword
"generic" @keyword
"derive" @keyword
"if" @keyword

; Module and import names
(module_identifier) @module
(module_name (module_identifier) @module)

; Types and constructors
(type_signature name: (signature_name (identifier) @type))
(type_definition name: (constructor) @type)
(class_declaration name: (class_name (constructor) @type))
(class_name (constructor) @type)
(constructor) @constructor

; Functions
(function_declaration name: (identifier) @function)
(macro_definition name: (identifier) @function)

; Variables
(identifier) @variable

; Literals
(string) @string
(number) @number
(char) @string
(integer) @number

; Comments
(line_comment) @comment
(block_comment) @comment

; Operators
(operator) @operator
(operator_add) @operator
(operator_mul) @operator
(operator_compare) @operator
(operator_cons) @operator
(operator_and) @operator
(operator_or) @operator

; Patterns
(wildcard) @keyword

; Punctuation
"=" @operator
(arrow) @operator
"," @punctuation.delimiter
"." @punctuation.delimiter
"|" @operator
"::" @operator
(comprehension_sep) @operator
