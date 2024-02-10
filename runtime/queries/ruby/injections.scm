((comment) @injection.content
 (#set! injection.language "comment"))

((heredoc_body 
  (heredoc_content) @injection.content
  (heredoc_end) @name
  (#set! injection.language "sql")) 
 (#eq? @name "SQL"))
