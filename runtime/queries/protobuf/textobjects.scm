(message (message_body) @class.inside) @class.around
(enum (enum_body) @class.inside) @class.around
(service (service_body) @class.inside) @class.around

(rpc (message_or_enum_type) @parameter.inside) @function.inside
(rpc (message_or_enum_type) @parameter.around) @function.around

(comment) @comment.inside
(comment)+ @comment.around
