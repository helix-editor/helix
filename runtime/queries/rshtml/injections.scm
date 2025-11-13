((html_text) @injection.content
  (#set! injection.language "html")
  (#set! injection.include-children)
  (#set! injection.combined))

((rust_text) @injection.content
   (#set! injection.language "rust")
   (#set! injection.include-children)
   (#not-match? @injection.content "^else"))

((comment_block) @injection.content
  (#set! injection.language "comment"))
