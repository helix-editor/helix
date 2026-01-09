((heredoc_body
  (literal_content) @injection.content
  (heredoc_end) @_name
  (#set! injection.language "sql"))
  (#eq? @_name "SQL"))

((heredoc_body
  (literal_content) @injection.content
  (heredoc_end) @_name
  (#set! injection.language "html"))
  (#eq? @_name "HTML"))
