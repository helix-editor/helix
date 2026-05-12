; Types

(class_identifier
  (identifier) @type)

(primitive_type) @type.builtin

((class_identifier
   . (identifier) @_first @type.builtin
   (identifier) @type.builtin)
  (#match? @_first "^(android|dalvik|java|kotlinx)$"))

((class_identifier
   . (identifier) @_first @type.builtin
   . (identifier) @_second @type.builtin
   (identifier) @type.builtin)
  (#eq? @_first "com")
  (#match? @_second "^(android|google)$"))

; Methods

(method_definition
  (method_signature (method_identifier) @function.method))

(expression
  (opcode) @_invoke
	(body
	  (full_method_signature
      (method_signature (method_identifier) @function.method)))
  (#match? @_invoke "^invoke"))

(expression
  (opcode) @_field_access
	(body
	  (field_identifier) @variable.other.member)
  (#match? @_field_access "^[is](get|put)-"))

(method_handle
  (full_method_signature
	(method_signature (method_identifier) @function.method)))

(custom_invoke
  . (identifier) @function.method
  (method_signature (method_identifier) @function.method))

(annotation_value
  (body
    (method_signature (method_identifier) @function.method)))

(annotation_value
  (body
    (full_method_signature
      (method_signature (method_identifier) @function.method))))

(field_definition
	(body
		(method_signature (method_identifier) @function.method)))

(field_definition
	(body
	  (full_method_signature
		  (method_signature (method_identifier) @function.method))))

((method_identifier) @constructor
  (#match? @constructor "^(<init>|<clinit>)$"))

"constructor" @constructor

; Fields

[
  (field_identifier)
  (annotation_key)
] @variable.other.member

((field_identifier) @constant
  (#match? @constant "^[%u_]*$"))

; Variables

(variable) @variable.builtin

(local_directive
  (identifier) @variable)

; Parameters

(parameter) @variable.parameter
(param_identifier) @variable.parameter

; Labels

[
  (label)
  (jmp_label)
] @label

; Operators

; (opcode) @keyword.operator

((opcode) @keyword.control.return
  (#match? @keyword.control.return "^return"))

((opcode) @keyword.control.conditional
  (#match? @keyword.control.conditional "^if"))

((opcode) @keyword.control.conditional
  (#match? @keyword.control.conditional "^cmp"))

((opcode) @keyword.control.exception
  (#match? @keyword.control.exception "^throw"))

[
  "="
  ".."
] @operator

; Keywords

[
  ".class"
  ".super"
  ".implements"
  ".field"
  ".end field"
  ".annotation"
  ".end annotation"
  ".subannotation"
  ".end subannotation"
  ".param"
  ".end param"
  ".parameter"
  ".end parameter"
  ".local"
  ".end local"
  ".restart local"
  ".registers"
  ".packed-switch"
  ".end packed-switch"
  ".sparse-switch"
  ".end sparse-switch"
  ".array-data"
  ".end array-data"
  ".enum"
  (prologue_directive)
  (epilogue_directive)
] @keyword

[
  ".source"
] @keyword.directive

[
  ".method"
  ".end method"
] @keyword.function

[
  ".catch"
  ".catchall"
] @keyword.control.exception

; Literals

(string) @string
(source_directive (string "\"" _ @string.special.url "\""))
(escape_sequence) @constant.character.escape

(character) @constant.character

"L" @punctuation

(line_directive (number) @comment) @comment
(".locals" (number) @comment) @comment

(number) @constant.numeric.integer

[
 (float)
 (NaN)
 (Infinity)
] @constant.numeric.float

(boolean) @constant.builtin.boolean

(null) @constant.builtin

; Misc

(annotation_visibility) @keyword.storage.modifier

(access_modifier) @keyword.storage.type

(array_type
  "[" @punctuation.special)

["{" "}"] @punctuation.bracket

["(" ")"] @punctuation.bracket

[
  "->"
  ","
  ":"
  ";"
  "@"
  "/"
] @punctuation.delimiter

; Comments

(comment) @comment

(class_definition
  (comment) @comment.block.documentation)
