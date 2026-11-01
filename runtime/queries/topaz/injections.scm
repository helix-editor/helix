; Tagged templates are opaque tokens; their delimiters cannot be excluded.
; Comment injection is supported without guessing an embedded content range.
([(line_comment) (block_comment)] @injection.content
  (#set! injection.language "comment"))
