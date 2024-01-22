; inherits: ecma

[
  (jsx_expression)
] @rainbow.scope

(jsx_opening_element ["<" ">"] @rainbow.bracket) @rainbow.scope
(jsx_closing_element ["</" ">"] @rainbow.bracket) @rainbow.scope
(jsx_self_closing_element ["<" "/>"] @rainbow.bracket) @rainbow.scope
