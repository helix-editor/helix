([(line_comment) (doc_comment)] @injection.content
 (#set! injection.language "comment"))

; Heredocs can specify a language: <<SQL,sql
; The heredoc_lang node captures the language name (e.g., "sql")
; The heredoc_content node contains the actual content to highlight
(heredoc
  (heredoc_lang) @injection.language
  (heredoc_content) @injection.content)
