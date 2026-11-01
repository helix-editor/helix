(function_declaration
  body: (block "{" (_)* @function.inside "}")) @function.around

(lambda_expression body: (_) @function.inside) @function.around
(record_declaration body: (record_body) @class.inside) @class.around
(enum_declaration body: (enum_body) @class.inside) @class.around
(parameter) @parameter.inside
(call_argument_list (_) @parameter.inside)
[(line_comment) (block_comment)] @comment.inside
(line_comment)+ @comment.around
(block_comment) @comment.around
