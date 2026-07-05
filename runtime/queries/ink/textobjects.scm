(knot_block (knot function: _) . _+ @function.inside) @function.around
(knot_block (knot !function) . _+ @class.inside) @class.around
(stitch_block (stitch) . _+ @class.inside) @class.around


(params ((param) @parameter.inside . ","? @parameter.around) @parameter.around)
(args ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(line_comment) @comment.inside
(line_comment)+ @comment.around

(block_comment) @comment.inside @comment.around

(((list_value_def) @entry.inside ) . ","? @entry.around) @entry.around

