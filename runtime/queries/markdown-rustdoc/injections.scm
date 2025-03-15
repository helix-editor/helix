; inherits: markdown

; In Rust, it is common to have documentation code blocks not specify the
; language, and it is assumed to be Rust if it is not specified.

(fenced_code_block
  (code_fence_content) @injection.content
  (#set! injection.language "rust")
  (#set! injection.include-unnamed-children))

; cargo-script uses an embedded virtual Cargo manifest
(fenced_code_block
  (info_string
    (language) @_language)
  (code_fence_content) @injection.content
(#eq? @_language "cargo") (#set! injection.language "toml") (#set! injection.include-unnamed-children))

(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content (#set! injection.include-unnamed-children))
  
