; See runtime/queries/ecma/README.md for more info.

(jsx_self_closing_element) @xml_element.around @xml_element.inside

(jsx_element (jsx_opening_element) (_)* @xml_element.inside (jsx_closing_element))

(jsx_element) @xml_element.around
