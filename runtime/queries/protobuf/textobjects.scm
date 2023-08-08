(rpc) @function.inside
(rpc) @function.around

(rpc
    (enumMessageType) @parameter.inside)

(message
    (messageBody) @class.inside) @class.around

(service
    (serviceBody) @class.inside) @class.around

(comment) @comment.inside
(comment)+ @comment.around
