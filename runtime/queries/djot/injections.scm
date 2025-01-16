((content) @injection.content
 (#set! injection.language "comment"))

(code_block
  (language) @injection.language
  (code) @injection.content (#set! injection.include-unnamed-children))

(raw_block
  (raw_block_info
    (language) @injection.language)
  (content) @injection.content (#set! injection.include-unnamed-children))

(raw_inline
  (content) @injection.content
  (raw_inline_attribute
    (language) @injection.language) (#set! injection.include-unnamed-children))
