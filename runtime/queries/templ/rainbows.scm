; inherits: go

[
  (expression)
  (component_switch_statement)
  (component_render)
  (component_children_expression)
  (doctype)
  (spread_attributes)
  (conditional_attribute_block)
  (css_declaration)
  (dynamic_class_attribute_value)
  (component_block)
  (script_block)
  (rawgo_block)
  (literal_value)
] @rainbow.scope

[
  "{"
  "}"
  "{!"
  "}"
  "{{"
  "}}"
  "<"
  ">"
  "</"
  "/>"
  "<!"
] @rainbow.bracket

([
    (element)
    (style_element)
    (script_element)
  ] @rainbow.scope
  (#set! rainbow.include-children))
