(script_element (start_tag) (_) @xml-element.inside (end_tag))  @xml-element.around

(style_element (start_tag) (_) @xml-element.inside (end_tag)) @xml-element.around

(element (start_tag) (_)* @xml-element.inside (end_tag))

(element) @xml-element.around

(comment) @comment.around
