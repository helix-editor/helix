; <script> content defaults to JavaScript
((element
  (start_tag (tag_name) @_tag)
  (raw_text) @injection.content)
  (#eq? @_tag "script")
  (#set! injection.language "javascript"))

; <style> content defaults to CSS
((element
  (start_tag (tag_name) @_tag)
  (raw_text) @injection.content)
  (#eq? @_tag "style")
  (#set! injection.language "css"))

; <script> or <style> with "lang" attribute
((element
  (start_tag
    (tag_name) @_tag
    (attribute
      (attribute_name) @_lang
      (quoted_attribute_value
        (attribute_value) @injection.language)))
  (raw_text) @injection.content)
  (#any-of? @_tag "style" "script")
  (#eq? @_lang "lang"))

; Inline style attribute
((attribute
  (attribute_name) @_style_name
  (quoted_attribute_value (attribute_value) @injection.content))
  (#eq? @_style_name "style")
  (#set! injection.language "css"))

; Typed expression content
((expression content: (js) @injection.content)
  (#set! injection.language "javascript"))
((expression content: (ts) @injection.content)
  (#set! injection.language "typescript"))

; Shorthand attributes ({foo})
((shorthand_attribute content: (js) @injection.content)
  (#set! injection.language "javascript"))
((shorthand_attribute content: (ts) @injection.content)
  (#set! injection.language "typescript"))

; Tag expressions ({@const}, {@render}, {@html}, {@debug}, {@attach}, {:else if})
((expression_value content: (js) @injection.content)
  (#set! injection.language "javascript"))
((expression_value content: (ts) @injection.content)
  (#set! injection.language "typescript"))

; Else-if clause
((else_if_clause expression: (expression_value content: (js) @injection.content))
  (#set! injection.language "javascript"))
((else_if_clause expression: (expression_value content: (ts) @injection.content))
  (#set! injection.language "typescript"))

; If block expressions
((if_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))
((if_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

; Each block expressions and bindings
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

; Await block expressions and bindings
((await_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))
((await_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))
((await_block (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))
((await_block (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))
((await_branch (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))
((await_branch (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))
((orphan_branch (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))
((orphan_branch (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))

; Key block expressions
((key_block expression: (expression content: (js) @injection.content))
  (#set! injection.language "javascript"))
((key_block expression: (expression content: (ts) @injection.content))
  (#set! injection.language "typescript"))

; Snippet parameters and type parameters
((snippet_parameters parameter: (pattern content: (js) @injection.content))
  (#set! injection.language "javascript"))
((snippet_parameters parameter: (pattern content: (ts) @injection.content))
  (#set! injection.language "typescript"))
((snippet_block type_parameters: (snippet_type_parameters) @injection.content)
  (#set! injection.language "typescript"))

; Comments
((comment) @injection.content
  (#set! injection.language "comment"))
