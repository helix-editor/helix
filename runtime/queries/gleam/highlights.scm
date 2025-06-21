; Variables
(identifier) @variable
(discard) @comment.unused ; `_` pattern
(hole) @comment.unused ; `_`, `_foo` unused variable

; Comments
(module_comment) @comment.line.documentation
(statement_comment) @comment.line.documentation
(comment) @comment.line

; Constants
(constant
  name: (identifier) @constant)

; Modules
(module) @namespace
(import alias: (identifier) @namespace)
(remote_type_identifier
  module: (identifier) @namespace)
(remote_constructor_name
  module: (identifier) @namespace)
((field_access
  record: (identifier) @namespace
  field: (label) @function)
 (#is-not? local))

; =========
; Functions
; =========

(unqualified_import (identifier) @function)
(unqualified_import "type" (type_identifier) @type)
(unqualified_import (type_identifier) @constructor)
(function
  name: (identifier) @function)
(external_function
  name: (identifier) @function)
(function_parameter
  name: (identifier) @variable.parameter)
((function_call
   function: (identifier) @function)
 (#is-not? local))
; highlights `a` in `|> a` as function
((binary_expression
   operator: "|>"
   right: (identifier) @function)
 (#is-not? local))

; =========
; Misc
; =========

; "Properties"
; Assumed to be intended to refer to a name for a field; something that comes
; before ":" or after "."
; e.g. record field names, tuple indices, names for named arguments, etc
(label) @variable.other.member
(tuple_access
  index: (integer) @variable.other.member)

; Attributes
(attribute
  "@" @attribute
  name: (identifier) @attribute)

(attribute_value (identifier) @constant)

; =========
; Types
; =========

(type_hole) @comment.unused

; Type names
(remote_type_identifier) @type
(type_identifier) @type

; Generic types
[
  ; in `pub type Dict(key, value)` this is `key` and `value`
  (type_parameter)
  ; in `pub fn size(dict: Dict(key, value)) -> Int` this is `key` and `value`
  (type_var)
] @type

; Data constructors
(constructor_name) @constructor

; built-ins
((constructor_name) @constant.builtin
  (#any-of? @constant.builtin "False" "True"))
((constructor_name) @constant.builtin
  (#any-of? @constant.builtin "Nil"))
((constructor_name) @type.enum.variant.builtin
  (#any-of? @type.enum.variant.builtin "Ok" "Error" "Some" "None"))

; =========
; Literals
; =========

(string) @string
(escape_sequence) @constant.character.escape
((escape_sequence) @warning
 (#eq? @warning "\\e")) ; deprecated escape sequence
(bit_string_segment_option) @function.builtin
(integer) @constant.numeric.integer
(float) @constant.numeric.float

; Reserved identifiers
((identifier) @error
 (#any-of? @error "auto" "delegate" "derive" "else" "implement" "macro" "test"))

; =========
; Keywords
; =========

[
  (visibility_modifier) ; "pub"
  (opacity_modifier) ; "opaque"
  "as"
  "assert"
  "case"
  "const"
  ; DEPRECATED: 'external' was removed in v0.30.
  "external"
  "fn"
  "if"
  "import"
  "let"
  "panic"
  "todo"
  "type"
  "use"
  "echo"
] @keyword

; =========
; Operators
; =========

(binary_expression
  operator: _ @operator)
(boolean_negation "!" @operator)
(integer_negation "-" @operator)

[
  "->"
  "-"
  "="
  ".."
  "<-"
  ; OR clause in patterns
  "|"
] @operator

; ==========
; Punctuation
; ==========

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<<"
  ">>"
] @punctuation.bracket

(tuple_type "#" @punctuation.bracket)
(tuple "#" @punctuation.bracket)
(tuple_pattern "#" @punctuation.bracket)

[
  ","
  ":"
] @punctuation.delimiter

; the `/` in `import gleam/list`
(import (module "/" @punctuation.delimiter))

[
  "."
] @punctuation

; affects e.g. `replace` in `string.replace("+", "-")`
; without this, it would be highlighted as a field instead of function
(function_call (field_access (label) @function))

; highlights `floor` in `|> float.floor` as function
(binary_expression
  left: (_) "|>"
  right: (field_access
    record: (identifier) "."
    field: (label) @function))
