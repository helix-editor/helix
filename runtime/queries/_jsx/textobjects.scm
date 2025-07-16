; See runtime/queries/ecma/README.md for more info.

(jsx_self_closing_element) @xml-element.around @xml-element.inside

(jsx_element (jsx_opening_element) (_)* @xml-element.inside (jsx_closing_element))

(jsx_element) @xml-element.around
