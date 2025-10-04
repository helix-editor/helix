[
  (comment)+
  (header_comment)+
] @comment.around
[
  (comment)
  (header_comment)
] @comment.inside
(formal_arguments) @parameter.around
(entity_declaration_group) @parameter.inside
(attribute_or_routine) @function.around
(feature_body) @function.inside
(class_declaration) @class.around
(feature_clause) @class.inside

