; Assume all-caps names are constants
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]+$'"))

; Function definitions/declarations
(function_definition
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)
(parameter_declaration
  name: (identifier) @variable.parameter)

; Methods / Properties
(field_access
  field: (identifier) @variable.other.member)

; Function calls
(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_access
    field: (identifier) @function))

; Types
(builtin_type) @type.builtin
(type (identifier) @type)
(any_type) @type

; Variables
(variable_storage_class) @keyword.storage
(variable_declaration
  name: (identifier) @variable)
(old_variable_declaration
  name: (identifier) @variable)

; Preprocessor
(preproc_include) @keyword.control.import
(preproc_tryinclude) @keyword.control.import
(system_lib_string) @string
(string_literal) @string

(preproc_assert) @keyword.directive
(preproc_pragma) @keyword.directive
(preproc_arg) @constant
(preproc_macro) @function.macro
(macro_param) @variable.parameter
(preproc_if) @keyword.directive
(preproc_else) @keyword.directive
(preproc_elseif) @keyword.directive
(preproc_endif) @keyword.directive
(preproc_endinput) @keyword.directive
(preproc_define) @keyword.directive
(preproc_define
  name: (identifier) @constant)
(preproc_undefine) @keyword.directive
(preproc_undefine
  name: (identifier) @constant)
(preproc_error) @function.macro ; Wrong color?
(preproc_warning) @function.macro ; Wrong color?

; Statements
(for_statement) @keyword.control.repeat
(condition_statement) @keyword.control.conditional
(while_statement) @keyword.control.repeat
(do_while_statement) @keyword.control.repeat
(switch_statement) @keyword.control.conditional
(switch_case) @keyword.control.conditional
(ternary_expression) @conditional.ternary

; Expressions
(view_as) @function.builtin
(sizeof_expression) @function.macro
(this) @variable.builtin

; https://github.com/alliedmodders/sourcemod/blob/5c0ae11a4619e9cba93478683c7737253ea93ba6/plugins/include/handles.inc#L78
(hardcoded_symbol) @variable.builtin

; Comments
(comment) @comment

; General
(parameter_declaration
  defaultValue: (identifier) @constant)
(fixed_dimension) @punctuation.bracket ; the [3] in var[3]
(dimension) @punctuation.bracket
(array_indexed_access) @punctuation.bracket
(escape_sequence) @constant.character.escape

; Constructors
(new_expression
  class: (identifier) @type
  arguments: (call_arguments) @constructor)

; Methodmaps
(methodmap) @type.definition
(methodmap
  name: (identifier) @type)
(methodmap
  inherits: (identifier) @type)
(methodmap_method_constructor
  name: (identifier) @constructor)
(methodmap_method
  name: (identifier) @function.method)
(methodmap_native
  name: (identifier) @function.method)
(methodmap_property
  name: (identifier) @variable.other.member)
(methodmap_property_getter) @function.method
(methodmap_property_setter) @function.method

; Enum structs
(enum_struct) @type.enum.variant
(enum_struct
  name: (identifier) @type)
(enum_struct_field
  name: (identifier) @variable.other.member)
(enum_struct_method
  name: (identifier) @function.method)

; Non-type Keywords
(variable_storage_class) @keyword.storage
(visibility) @keyword.storage
(visibility) @keyword.storage
(assertion) @function.builtin
(function_declaration_kind) @keyword.function
[
  "new"
  "delete"
] @keyword.operator
[
  "."
  ","
] @punctuation.delimiter

; Operators
[
  "+"
  "-"
  "..."
  "*"
  "/"
  "%"
  "++"
  "--"
  "="
  "+="
  "-="
  "*="
  "/="
  "=="
  "!="
  "<"
  ">"
  ">="
  "<="
  "!"
  "&&"
  "||"
  "&"
  "|"
  "~"
  "^"
  "<<"
  ">>"
  ">>>"
  "|="
  "&="
  "^="
  "~="
  "<<="
  ">>="
] @operator
(ignore_argument) @operator
(scope_access) @operator
(rest_operator) @operator

; public Plugin myinfo
(struct_declaration
  name: (identifier) @variable.builtin)

; Typedef/Typedef
(typeset) @type.builtin
(typedef) @type.builtin
(functag) @type.builtin
(funcenum) @type.builtin
(typedef_expression) @keyword.function ; function void(int x)

; Enums
(enum) @type.enum
(enum
  name: (identifier) @type)
(enum_entry
  name: (identifier) @constant)
(enum_entry
  value: (_) @constant)

; Literals
(int_literal) @constant.numeric.integer
(char_literal) @constant.character
(float_literal) @constant.numeric.float
(string_literal) @string
(array_literal) @punctuation.bracket
[
  (bool_literal)
  (null)
] @constant.builtin
((identifier) @constant
  (#match? @constant "INVALID_HANDLE"))

; Comment specialisations (must be after comment)
; These might be unnecessary and/or used incorrectly, since they're intended
; for markup languages
((comment) @diff.plus
  (#match? @diff.plus "^\/[\/\*][\t ]TODO"))
((comment) @diff.plus
  (#match? @diff.plus "^\/[\/\*][\t ]NOTE"))
((comment) @diff.minus
  (#match? @diff.minus "^\/[\/\*][\t ]WARNING"))

; Keywords
[
  "__nullable__"
  "break"
  "case"
  "const"
  "continue"
  "default"
  "delete"
  "do"
  "else"
  "enum"
  "for"
  "forward"
  "funcenum"
  "functag"
  "get"
  "if"
  "methodmap"
  "native"
  "new"
  "property"
  "public"
  "return"
  "set"
  "static"
  "stock"
  "struct"
  "switch"
  "typedef"
  "typeset"
  "void"
  "while"
] @keyword

(identifier) @variable
