
((html_tag) @injection.content 
  (#set! injection.language "html") 
  (#set! injection.include-unnamed-children)
  (#set! injection.combined))

((latex_block) @injection.content (#set! injection.language "latex") (#set! injection.include-unnamed-children))
