; Function definitions
(function_declaration
  name: (identifier) @function.around) @function.around

; Class definitions
(class_declaration
  name: (class_name) @class.around) @class.around

; Instance definitions
(instance_declaration
  name: (instance_class) @class.around) @class.around

; Comments
(line_comment) @comment.inside
(block_comment) @comment.inside
(line_comment)+ @comment.around
(block_comment) @comment.around
