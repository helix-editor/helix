(grammar_rule (_) @class.inside) @class.around
(term (_) @entry.inside) @entry.around

(line_comment) @comment.inside
(line_comment)+ @comment.around

(block_comment) @comment.inside
(block_comment)+ @comment.around
