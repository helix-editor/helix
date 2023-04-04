((comment) @injection.content
 (#set! injection.language "comment"))

((heredoc_beginning) @name 
 (heredoc_body 
  (heredoc_content) @injection.content
   (#set! injection.language "sql")) 
 (#eq? @name "<<~SQL"))