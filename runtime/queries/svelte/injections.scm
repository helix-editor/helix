; inherits html

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (attribute_name) @_lang
      (quoted_attribute_value (attribute_value) @injection.language)))
  (raw_text) @injection.content)
  (#eq? @_tag "script")
  (#eq? @_lang "lang"))

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (quoted_attribute_value (attribute_value) @_value)))
  (raw_text) @injection.content)
  (#match? @_tag "^[Ss][Cc][Rr][Ii][Pp][Tt]$")
  (#any-of? @_value "ts" "typescript" "text/typescript")
  (#set! injection.language "typescript"))

((element
  (start_tag (tag_name) @_tag)
  (raw_text) @injection.content)
  (#eq? @_tag "script")
  (#set! injection.language "javascript"))

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (attribute_name) @_lang
      (quoted_attribute_value (attribute_value) @injection.language)))
  (raw_text) @injection.content)
  (#eq? @_tag "style")
  (#eq? @_lang "lang"))

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (quoted_attribute_value (attribute_value) @_scss)))
  (raw_text) @injection.content)
  (#eq? @_tag "style")
  (#eq? @_scss "scss")
  (#set! injection.language "scss"))

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (quoted_attribute_value (attribute_value) @_sass)))
  (raw_text) @injection.content)
  (#eq? @_tag "style")
  (#eq? @_sass "sass")
  (#set! injection.language "sass"))

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (quoted_attribute_value (attribute_value) @_less)))
  (raw_text) @injection.content)
  (#eq? @_tag "style")
  (#eq? @_less "less")
  (#set! injection.language "less"))

((element
  (start_tag (tag_name) @_tag)
  (raw_text) @injection.content)
  (#eq? @_tag "style")
  (#set! injection.language "css"))

((attribute
  (attribute_name) @_name
  (quoted_attribute_value) @injection.content)
  (#eq? @_name "style")
  (#set! injection.language "css")
  (#set! injection.include-children))

((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (attribute_name) @_lang
      (quoted_attribute_value (attribute_value) @injection.language)))
  (text) @injection.content)
  (#eq? @_tag "template")
  (#eq? @_lang "lang"))

((expression content: (js) @injection.content)
  (#set! injection.language "javascript"))

((expression content: (ts) @injection.content)
  (#set! injection.language "typescript"))

((shorthand_attribute content: (js) @injection.content)
  (#set! injection.language "javascript"))

((shorthand_attribute content: (ts) @injection.content)
  (#set! injection.language "typescript"))

((expression_value content: (js) @injection.content)
  (#set! injection.language "javascript"))

((expression_value content: (ts) @injection.content)
  (#set! injection.language "typescript"))

((else_if_clause expression: (expression_value content: (js) @injection.content))
  (#set! injection.language "javascript"))

((else_if_clause expression: (expression_value content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((if_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))

((if_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((each_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))

((each_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((each_block binding: (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))

((each_block binding: (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((each_block index: (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))

((each_block index: (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((each_block key: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))

((each_block key: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((await_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))

((await_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((await_branch (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))

((await_branch (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((await_block (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))

((await_block (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((orphan_branch (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))

((orphan_branch (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((key_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))

((key_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((snippet_parameters parameter: (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))

((snippet_parameters parameter: (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

((snippet_block type_parameters: (snippet_type_parameters) @injection.content)
  (#set! injection.language "typescript"))

((comment) @injection.content
  (#set! injection.language "comment"))
