; inherits: go

(style_element) @xml-element.around
(style_element
  [(self_closing_style_tag) (style_element_text)] @xml-element.inside)

(script_element) @xml-element.around
(script_element
  [(script_element_text) (self_closing_script_tag)] @xml-element.inside)

(element) @xml-element.around
(element (self_closing_tag) @xml-element.inside)
(element (tag_start) (_)* @xml-element.inside (tag_end))

(element_comment) @comment.around

(component_declaration
  (component_block) @function.inside) @function.around

; TODO: function.inside textobjects
(css_declaration) @function.around

(script_declaration
  (script_block) @function.inside) @function.around
