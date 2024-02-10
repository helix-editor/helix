((xhtml_tag) @injection.content
 (#set! injection.combined)
 (#set! injection.include-children)
 (#set! injection.language "html"))

((block_quote
   !language
   (quote_content) @injection.content)
  (#set! injection.language "erlang"))

(block_quote
  language: (language_identifier) @injection.language
  (quote_content) @injection.content)

((macro
   (tag) @_tag
   (argument) @injection.content)
 (#eq? @_tag "@type")
 (#set! injection.language "erlang")
 (#set! injection.include-children))
