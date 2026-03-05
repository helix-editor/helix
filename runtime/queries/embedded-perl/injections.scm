((code_tag (tag_content) @injection.content)
 (#set! injection.language "perl"))

((expression_tag (tag_content) @injection.content)
 (#set! injection.language "perl"))

((raw_expression_tag (tag_content) @injection.content)
 (#set! injection.language "perl"))

((line_code (line_content) @injection.content)
 (#set! injection.language "perl"))

((line_expression (line_content) @injection.content)
 (#set! injection.language "perl"))

((line_raw_expression (line_content) @injection.content)
 (#set! injection.language "perl"))

((non_directive_text) @injection.content
 (#set! injection.language "html")
 (#set! injection.combined))
