(markdown
  (markdown_line) @injection.content
  (#set! injection.language "markdown")
  (#set! injection.combined))

(code_block
  (code_language) @injection.language
  (code_line) @injection.content
  (#set! injection.combined))
