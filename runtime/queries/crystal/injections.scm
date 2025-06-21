((heredoc_body
  (literal_content) @injection.content
  (heredoc_end) @name
  (#set! injection.language "sql"))
  (#eq? @name "SQL"))

((heredoc_body
  (literal_content) @injection.content
  (heredoc_end) @name
  (#set! injection.language "html"))
  (#eq? @name "HTML"))
