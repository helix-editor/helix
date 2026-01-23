(comment) @comment.inside
(comment)+ @comment.around

(directive
  name: (directive_name) @parameter.inside) @parameter.around

(global_options
  "{" (_)* @class.inside "}") @class.around

(snippet_definition
  (block) @class.inside) @class.around

(named_route
  (block) @class.inside) @class.around

(site_definition (block) @class.inside) @class.around
