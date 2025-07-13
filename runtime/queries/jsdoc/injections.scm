; Parse general comment tags

((document) @injection.content
 (#set! injection.include-children)
 (#set! injection.language "comment"))