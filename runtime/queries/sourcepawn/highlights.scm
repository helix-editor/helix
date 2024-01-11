(identifier) @variable
; Assume all-caps names are constants
((identifier) @constant
  (#lua-match? @constant "^[A-Z][A-Z0-9_]+$"))

; Function definitions/declarations
(function_definition
  name: (identifier) @function)

(function_declaration
  name: (identifier) @function)

(parameter_declaration
  name: (identifier) @variable.parameter)

; Methods / Properties
(field_access
  field: (identifier) @variable.member)

; Function calls
(call_expression
  function: (identifier) @function)

(call_expression
  function:
    (field_access
      field: (identifier) @function.method.call)) ; Must be after field_access

; Types
(builtin_type) @type.builtin

(type
  (identifier) @type)

(any_type) @type

; Variables
(variable_storage_class) @keyword.storage

(variable_declaration
  name: (identifier) @variable)

(old_variable_declaration
  name: (identifier) @variable)

; Preprocessor
(preproc_include) @keyword.import

(preproc_tryinclude) @keyword.import

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

(preproc_define) @keyword.directive.define

(preproc_define
  name: (identifier) @constant)

(preproc_undefine) @keyword.directive.define

(preproc_undefine
  name: (identifier) @constant)

(preproc_error) @function.macro

(preproc_warning) @function.macro

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

(fixed_dimension) @punctuation.bracket

(dimension) @punctuation.bracket

(array_indexed_access) @punctuation.bracket

(escape_sequence) @string.escape

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
  name: (identifier) @property)

(methodmap_property_getter) @function.method

(methodmap_property_setter) @function.method

; Enum structs
(enum_struct) @type.definition

(enum_struct
  name: (identifier) @type)

(enum_struct_field
  name: (identifier) @variable.member)

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

(rest_operator) @operator ; Should override (concatenated_string) but currently does nothing

; public Plugin myinfo
(struct_declaration
  name: (identifier) @variable.builtin)

; Typedef/Typedef
(typeset) @type.definition

(typedef) @type.definition

(functag) @type.definition

(funcenum) @type.definition

(typedef_expression) @keyword.function ; function void(int x)

; Enums
(enum) @type.definition

(enum
  name: (identifier) @type)

(enum_entry
  name: (identifier) @constant)

(enum_entry
  value: (_) @constant)

; Literals
(int_literal) @number

(char_literal) @character

(float_literal) @number.float

(string_literal) @string

(array_literal) @punctuation.bracket

(concatenated_string) @punctuation.delimiter

(bool_literal) @boolean

(null) @constant.builtin

((identifier) @constant
  (#eq? @constant "INVALID_HANDLE"))

; Keywords
"return" @keyword.return
[
  "if"
  "else"
  "case"
  "default"
  "switch"
] @keyword.conditional
[
  "do"
  "while"
  "for"
  "continue"
  "break"
] @keyword.repeat
[
  "__nullable__"
  "delete"
  "enum"
  "funcenum"
  "functag"
  "get"
  "methodmap"
  "new"
  "property"
  "public"
  "set"
  "struct"
  "typedef"
  "typeset"
  "void"
] @keyword

[
  "const"
  "native"
  "static"
  "stock"
  "forward"
] @type.qualifier