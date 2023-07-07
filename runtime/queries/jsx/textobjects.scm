; inherits: ecma
 
(jsx_fragment 
  (_)* @xml_element.inside 
) @xml_element.around 
 
((jsx_opening_element) 
    (_)* @xml_element.inside 
  (jsx_closing_element)
)

(jsx_element) @xml_element.around

(jsx_self_closing_element) @xml_element.around 