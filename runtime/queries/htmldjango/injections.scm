((content) @injection.content
 (#set! injection.language "html")
 (#set! injection.combined))

([(unpaired_comment) (paired_comment)] @injection.content
 (#set! injection.language "comment"))
