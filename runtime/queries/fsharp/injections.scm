([
 (line_comment)
 (block_comment_content)
] @injection.content
  (#set! injection.language "comment"))

((xml_doc) @injection.content
 (#set! injection.language "xml"))
