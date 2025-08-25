(function_declaration
  (function_body)? @function.inside) @function.around

; Unlike function_body above, the constructor body is does not have its own
; symbol in the current grammar.
(secondary_constructor) @function.around

(class_declaration
  (class_body)? @class.inside) @class.around

(class_declaration
  (enum_class_body) @class.inside) @class.around

[
  (line_comment)
  (multiline_comment)
] @comment.inside

(line_comment)+ @comment.around

(multiline_comment) @comment.around

(enum_entry) @entry.around
(lambda_literal) @entry.around
(property_declaration) @entry.around
(object_declaration) @entry.around
(assignment) @entry.around

; TODO: This doesn't work with annotations yet, but fixing it without breaking
; the case of multiple parameters is non-trivial.
(function_value_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

; secondary constructor uses function_value_parameters above
(primary_constructor
  ((_)@parameter.inside . ","? @parameter.around) @parameter.around)

(function_type_parameters
  ((_)@parameter.inside . ","? @parameter.around) @parameter.around)

(value_arguments
  ((_)@parameter.inside . ","? @parameter.around) @parameter.around)
