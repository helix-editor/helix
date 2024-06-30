; Comments
((line_comment) @injection.content
 (#set! injection.language "comment"))

((block_comment) @injection.content
 (#set! injection.language "comment"))

((xml_doc
   (xml_doc_content) @injection.content)
 (#set! injection.language "comment"))