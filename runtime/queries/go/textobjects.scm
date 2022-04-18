(function_declaration
  body: (block)? @function.inside) @function.around

(func_literal
  (_)? @function.inside) @function.around

(method_declaration
  body: (block)? @function.inside) @function.around

;; struct and interface declaration as class textobject?
(type_declaration
  (type_spec (type_identifier) (struct_type (field_declaration_list (_)?) @class.inside))) @class.around

(type_declaration
  (type_spec (type_identifier) (interface_type (method_spec)+ @class.inside))) @class.around

(parameter_list
  (_) @parameter.inside)

(argument_list
  (_) @parameter.inside)

(comment) @comment.inside

(comment)+ @comment.around
