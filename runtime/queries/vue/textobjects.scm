; Vue SFC blocks and template elements as xml-element textobjects, mirroring
; html. <script>/<style> bodies are a single raw_text child; template elements
; can hold many children.
(script_element (start_tag) (_) @xml-element.inside (end_tag)) @xml-element.around

(style_element (start_tag) (_) @xml-element.inside (end_tag)) @xml-element.around

(template_element (start_tag) (_)* @xml-element.inside (end_tag)) @xml-element.around

(element (start_tag) (_)* @xml-element.inside (end_tag))

(element) @xml-element.around

(comment) @comment.around
