; inherits: html

((frontmatter
	(raw_text) @injection.content)
 (#set! injection.language "typescript"))

((interpolation
	(raw_text) @injection.content)
 (#set! injection.language "tsx"))
