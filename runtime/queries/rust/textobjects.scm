(
  [
    (attribute_item)+
    (line_comment)+
  ]*
  .
  (function_item
    body: (_) @function.inside)) @function.around

(
  [
    (attribute_item)+
    (line_comment)+
  ]*
  .
  (struct_item
    body: (_) @class.inside)) @class.around

(
  [
    (attribute_item)+
    (line_comment)+
  ]*
  .
  (enum_item
    body: (_) @class.inside)) @class.around

(
  [
    (attribute_item)+
    (line_comment)+
  ]*
  .
  (union_item
    body: (_) @class.inside)) @class.around

(
  [
    (attribute_item)+
    (line_comment)+
  ]*
  .
  (trait_item
    body: (_) @class.inside)) @class.around

(
  [
    (attribute_item)+
    (line_comment)+
  ]*
  .
  (impl_item
    body: (_) @class.inside)) @class.around

(parameters 
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(type_arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(closure_parameters
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(arguments
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

[
  (line_comment)
  (block_comment)
] @comment.inside

(line_comment)+ @comment.around

(block_comment) @comment.around
