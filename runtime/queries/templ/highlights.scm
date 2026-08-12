; inherits: go

(css_declaration
  name: (css_identifier) @function)
(script_declaration
  name: (script_identifier) @function)
(component_declaration
  name: (component_identifier) @function)

; Elements

(tag_start name: (_) @tag)
(tag_end name: (_) @tag)
(self_closing_tag name: (_) @tag)

(tag_start ["<" ">"] @punctuation.bracket)
(tag_end ["</" ">"] @punctuation.bracket)
(self_closing_tag ["<" "/>"] @punctuation.bracket)

(style_tag_start "style" @tag)
(style_tag_end "style" @tag)
(self_closing_style_tag "style" @tag)

(style_tag_start ["<" ">"] @punctuation.bracket)
(style_tag_end ["</" ">"] @punctuation.bracket)
(self_closing_style_tag ["<" "/>"] @punctuation.bracket)

(script_tag_start "script" @tag)
(script_tag_end "script" @tag)
(self_closing_script_tag "script" @tag)

(script_tag_start ["<" ">"] @punctuation.bracket)
(script_tag_end ["</" ">"] @punctuation.bracket)
(self_closing_script_tag ["<" "/>"] @punctuation.bracket)

; Attributes

(attribute
  name: (attribute_name) @attribute)
(attribute
  value: (quoted_attribute_value) @string)

(css_property
  name: (css_property_name) @variable.other.member)
(css_property
  value: (css_property_value) @constant)

(dynamic_class_attribute_value) @function.method

; Special Elements and Attributes

((attribute
  name: (attribute_name) @attribute
  value: (quoted_attribute_value (attribute_value) @markup.link.url))
 (#any-of? @attribute "href" "src"))

((element
  (tag_start
    name: (_) @tag)
  (element_text) @markup.link.label)
  (#eq? @tag "a"))

((element
  (tag_start
    name: (_) @tag)
  (element_text) @markup.bold)
  (#any-of? @tag "strong" "b"))

((element
  (tag_start
    name: (_) @tag)
  (element_text) @markup.italic)
  (#any-of? @tag "em" "i"))

((element
  (tag_start
    name: (_) @tag)
  (element_text) @markup.strikethrough)
  (#any-of? @tag "s" "del"))

; Extra Features

(component_import
  name: (component_identifier) @function)

(component_render) @function

"@" @operator

[
  "templ"
  "css"
  "type"
] @keyword.storage.type
(script_declaration "script" @keyword.storage.type)

["{{" "}}"] @punctuation.bracket

; Comments

(element_comment) @comment
