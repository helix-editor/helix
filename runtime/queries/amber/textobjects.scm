; Functions - capture both definition and body
(function_definition
  body: (_) @function.inside) @function.around

; Function parameters in definitions
(function_parameter_list
  (function_parameter_list_item) @parameter.inside)

; Function call arguments
(parameter_list
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

; Comments
(comment) @comment.inside
(comment)+ @comment.around

; Arrays
(array
  (_) @entry.around)

; Main Block looks like a function
(main_block
  (block) @function.inside) @function.around
