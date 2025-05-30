(frontmatter (content) @injection.content
  (#set! injection.language "yaml")
  (#set! injection.combined)
)

((content) @injection.content
  (#set! injection.language "<use-2nd-filename-extension>")
  (#set! injection.combined))
