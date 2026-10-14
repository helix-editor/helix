(function_declaration name: (identifier) @function.around) @function.around
(class_declaration name: (class_name) @class.around) @class.around
(line_comment) @comment.inside
(block_comment) @comment.inside
(line_comment)+ @comment.around
(block_comment) @comment.around
