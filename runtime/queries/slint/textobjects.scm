(function_definition
  (imperative_block) @funtion.inside) @function.around

(callback_event
  (imperative_block) @function.inside) @function.around

(property
  (imperative_block) @function.inside) @function.around

(struct_definition
  (struct_block) @class.inside) @class.around

(enum_definition
  (enum_block) @class.inside) @class.around

(global_definition
  (global_block) @class.inside) @class.around

(component_definition
  (block) @class.inside) @class.around

(component_definition
  (block) @class.inside) @class.around

(comment) @comment.around

(typed_identifier
  name: (_) @parameter.inside) @parameter.around

(callback
  arguments: (_) @parameter.inside)

(string_value
  "\"" . (_) @text.inside . "\"") @text.around

