((comment) @injection.content
 (#set! injection.language "comment"))

((heredoc_body 
  (heredoc_content) @injection.content
  (heredoc_end) @name
  (#set! injection.language "sql")) 
 (#eq? @name "SQL"))

((heredoc_body
  (heredoc_content) @injection.content
  (heredoc_end) @name
  (#set! injection.language "graphql"))
 (#any-of? @name
       "GQL"
       "GRAPHQL"))

((heredoc_body
  (heredoc_content) @injection.content
  (heredoc_end) @name
  (#set! injection.language "erb"))
 (#eq? @name "ERB"))

