((javascript) @injection.content
  (#set! injection.language "javascript")
)

(
  script_tag
  ((content_code) @injection.content
    (#set! injection.language "javascript"))
)

(
  style_tag
  ((content_code) @injection.content
    (#set! injection.language "css"))
)
