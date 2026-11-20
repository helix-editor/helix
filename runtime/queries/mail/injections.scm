; Inject MIME parts based on their Content-Type subtype. A subtype that does
; not correspond to a configured Helix language is left uninterpreted.
(mime_part
  (part_headers
    (content_type_field
      value: (content_type
        subtype: (mime_subtype) @injection.language)))
  (part_body) @injection.content
  (#not-any-of? @injection.language "plain" "md" "x-markdown" "rtf" "x-rtf"))

; MIME aliases whose subtype does not match Helix's language name.
(mime_part
  (part_headers
    (content_type_field
      value: (content_type
        subtype: (mime_subtype) @_subtype)))
  (part_body) @injection.content
  (#any-of? @_subtype "md" "x-markdown")
  (#set! injection.language "markdown"))

(mime_part
  (part_headers
    (content_type_field
      value: (content_type
        subtype: (mime_subtype) @_subtype)))
  (part_body) @injection.content
  (#any-of? @_subtype "rtf" "x-rtf")
  (#set! injection.language "rtf"))

; Diff-bearing plain-text bodies receive diff highlighting; other text bodies
; are rendered as Markdown.
([
  (body_block)
  (epilogue)
] @injection.content
 (#match? @injection.content "diff --git ")
 (#set! injection.language "diff"))

([
  (body_block)
  (epilogue)
] @injection.content
 (#not-match? @injection.content "diff --git ")
 (#set! injection.language "markdown"))

(mime_part
  (part_headers
    (content_type_field
      value: (content_type
        subtype: (mime_subtype) @_subtype)))
  (part_body) @injection.content
  (#match? @_subtype "^[Pp][Ll][Aa][Ii][Nn]$")
  (#match? @injection.content "diff --git ")
  (#set! injection.language "diff"))

(mime_part
  (part_headers
    (content_type_field
      value: (content_type
        subtype: (mime_subtype) @_subtype)))
  (part_body) @injection.content
  (#match? @_subtype "^[Pp][Ll][Aa][Ii][Nn]$")
  (#not-match? @injection.content "diff --git ")
  (#set! injection.language "markdown"))
