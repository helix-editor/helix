([(line_comment) (block_comment)] @injection.content
 (#set! injection.language "comment"))

((sql_expression) @injection.content
 (#set! injection.language "sql"))

