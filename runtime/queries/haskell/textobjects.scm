(comment) @comment.inside
(comment)+ @comment.around

(newtype
	(newtype_constructor
		(_) @class.inside)) @class.around
(data_type
	constructors: (_) @class.inside) @class.around
(decl/function
	(match expression:(_) @function.inside)) @function.around
(lambda
	expression:(_) @function.inside) @function.around

(decl/function
	patterns: (patterns
		(_) @parameter.inside))

(expression/lambda
	patterns: (patterns
	(_) @parameter.inside))

(decl/function
	(infix
		(pattern) @parameter.inside))
