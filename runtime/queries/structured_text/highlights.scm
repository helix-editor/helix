; Identifiers and Variables
(identifier) @variable
(variable name: (identifier) @variable)
(builtin_variable) @variable.builtin

; Functions and Methods
(program_definition programName: (identifier) @function)
(action_definition ActionName: (identifier) @function)
(function_definition functionName: (identifier) @function)
(function_block_definition functionName: (identifier) @function)
(method_definition methodName: (identifier) @function.method)
(call_expression functionName: (identifier) @function.call)
(builtin_function) @function.builtin

; Types
(basic_data_type) @type.builtin
(type_definition typeName: (variable) @type)
(struct_definition typeName: (variable) @type)
(union_definition typeName: (variable) @type)
(enum_definition typeName: (variable) @type)
(pointer_type) @type
(reference_type) @type
(array_type) @type

; Parameters and Fields
(parameter_assignment (parameter) @parameter)
(structure_member (variable name: (identifier) @property))
(structure_member (call_expression functionName: (identifier) @function.method.call))

; Keywords - Declarations
[
  "PROGRAM"
  "END_PROGRAM"
  "ACTION"
  "END_ACTION"
  "FUNCTION"
  "END_FUNCTION"
  "FUNCTION_BLOCK"
  "END_FUNCTION_BLOCK"
  "METHOD"
  "END_METHOD"
  "TYPE"
  "END_TYPE"
  "STRUCT"
  "END_STRUCT"
  "UNION"
  "END_UNION"
  "VAR"
  "VAR_INPUT"
  "VAR_OUTPUT"
  "VAR_IN_OUT"
  "VAR_GLOBAL"
  "VAR_TEMP"
  "VAR_STAT"
  "VAR_EXTERNAL"
  "VAR_INST"
  "VAR_CONFIG"
  "VAR_ACCESS"
  "VAR_GENERIC"
  "END_VAR"
] @keyword

; Keywords - Statements and Control Flow
[
  "IF"
  "THEN"
  "ELSIF"
  "ELSE"
  "END_IF"
  "CASE"
  "OF"
  "END_CASE"
  "FOR"
  "TO"
  "BY"
  "DO"
  "END_FOR"
  "WHILE"
  "END_WHILE"
  "REPEAT"
  "UNTIL"
  "END_REPEAT"
  "EXIT"
  "CONTINUE"
  "RETURN"
] @keyword.control

; Keywords - Modifiers and Attributes
[
  "PUBLIC"
  "PROTECTED"
  "PRIVATE"
  "INTERNAL"
  "CONSTANT"
  "RETAIN"
  "PERSISTENT"
  "EXTENDS"
  "IMPLEMENTS"
] @keyword.modifier

; Literals
(boolean) @boolean
(integer) @number
(floating_point) @number.float
(binary) @number
(octal) @number
(hexidecimal) @number
(time) @string.special
(date) @string.special
(time_of_day) @string.special
(date_and_time) @string.special

; Strings
(string) @string
(wstring) @string
(escape_sequence) @string.escape

; Comments and Pragma
(doc_comment) @comment.documentation
(inline_comment) @comment
(block_comment) @comment
(pragma) @keyword.directive

; Operators
[
  ":="
  "+"
  "-"
  "*"
  "**"
  "/"
  "="
  "<"
  ">"
  "<="
  ">="
  "<>"
  ".."
] @operator

[
  "AND"
  "OR"
  "XOR"
  "NOT"
  "MOD"
  "DIV"
  "MUL"
  "LT"
  "GT"
  "LE"
  "GE"
  "EQ"
  "NE"
] @operator.word

; Punctuation
[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

[
  ":"
  ";"
  ","
  "."
] @punctuation.delimiter
