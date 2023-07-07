(script_element (start_tag) (_) @xml_element.inside (end_tag))  @xml_element.around

(style_element (start_tag) (_) @xml_element.inside (end_tag)) @xml_element.around 

(element (start_tag) (_)* @xml_element.inside (end_tag))

(element) @xml_element.around  

(comment) @comment.around   