((template_content) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))

((json_content) @injection.content
  (#set! injection.language "json")
  (#set! injection.combined))

((style_content) @injection.content
  (#set! injection.language "css")
  (#set! injection.combined))

((stylesheet_content) @injection.content
  (#set! injection.language "css")
  (#set! injection.combined))

((js_content) @injection.content
  (#set! injection.language "javascript")
  (#set! injection.combined))

((front_matter) @injection.content
  (#set! injection.language "yaml"))
