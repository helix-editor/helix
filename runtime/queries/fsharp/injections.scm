([
 (line_comment)
 (block_comment_content)
] @injection.content
  (#set! injection.language "comment"))

((xml_doc (xml_doc_content) @injection.content)
 (#set! injection.language "xml"))
