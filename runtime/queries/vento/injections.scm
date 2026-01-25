((comment) @injection.content
 (#set! injection.language "comment"))

((content) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))

((code) @injection.content
  (#set! injection.language "javascript"))
