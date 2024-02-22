(type (_) @test.inside) @test.around

(node
	children: (node_children)? @class.inside) @class.around

(node
	children: (node_children)? @function.inside) @function.around

(node (identifier) @function.movement)

[
	(single_line_comment)
	(multi_line_comment)
] @comment.inside

[
	(single_line_comment)+
	(multi_line_comment)+
] @comment.around

[
	(prop)
	(value)
] @parameter.inside

(value (type) ? (_) @parameter.inside @parameter.movement . ) @parameter.around

