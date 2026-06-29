(element (start_tag) (_)* @xml-element.inside (end_tag))

(element) @xml-element.around

(comment) @comment.around

(snippet_block
  parameters: (snippet_parameters) @parameter.inside) @function.around

(snippet_name) @function.inside
