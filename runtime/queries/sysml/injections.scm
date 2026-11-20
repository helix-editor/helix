([
  (line_note)
  (block_note)
  (regular_comment)
] @injection.content
  (#set! injection.language "comment"))

((textual_representation
  (string) @injection.language
  (regular_comment) @injection.content))
