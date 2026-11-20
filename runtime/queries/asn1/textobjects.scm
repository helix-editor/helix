(line_comment) @comment.inside
(line_comment)+ @comment.around
(block_comment) @comment.inside
(block_comment)+ @comment.around
(NamedType) @entry.inside
(ComponentType) @entry.inside @entry.around
(ActualParameter) @parameter.inside
(ActualParameterList) @parameter.around
[
    (Type)
    (ObjectClass)
] @class.inside @class.around
