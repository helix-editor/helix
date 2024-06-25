(function (body) @function.inside) @function.around

(parameters ((_) @parameter.inside . ","? @parameter.around) @parameter.around)
(arguments ((_) @parameter.inside . ","? @parameter.around) @parameter.around) 

(comment) @comment.inside
(comment)+ @comment.around

(array (_) @entry.around)
(map (_) @entry.around)
(pair (_) @entry.inside) @entry.around
