(keyword) @keyword
(operator) @operator

(int_literal)   @number
(float_literal) @number
(rune_literal)  @number
(bool_literal) @boolean
(nil) @constant.builtin


(ERROR) @error

(type_identifier)    @type
(package_identifier) @namespace
(label_identifier)   @label

(interpreted_string_literal) @string
(raw_string_literal) @string
(escape_sequence) @string.escape

(comment) @comment
(const_identifier) @constant


(compiler_directive) @attribute
(calling_convention) @attribute

(identifier) @variable
(pragma_identifier) @attribute
