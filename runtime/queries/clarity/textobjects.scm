; Functions
(private_function
  (function_signature)
  (_) @function.inside) @function.around

(read_only_function
  (function_signature)
  (_) @function.inside) @function.around

(public_function
  (function_signature)
  (_) @function.inside) @function.around

; FIXME: trait_definition not matching — may be a grammar issue
; (trait_definition) @class.around

; Parameters
(function_signature
  (function_parameter) @parameter.inside @parameter.around)

; Comments
(comment) @comment.inside
(comment)+ @comment.around

; Entries
(local_binding) @entry.inside @entry.around
