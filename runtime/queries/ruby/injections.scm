((comment) @injection.content
 (#set! injection.language "comment"))

((heredoc_body 
  (heredoc_content) @injection.content
  (heredoc_end) @_name
  (#set! injection.language "sql")) 
 (#eq? @_name "SQL"))

((heredoc_body
  (heredoc_content) @injection.content
  (heredoc_end) @_name
  (#set! injection.language "graphql"))
 (#any-of? @_name
       "GQL"
       "GRAPHQL"))

((heredoc_body
  (heredoc_content) @injection.content
  (heredoc_end) @_name
  (#set! injection.language "erb"))
 (#eq? @_name "ERB"))

; `<command>`
; %x{<command>}
(subshell
  (string_content) @injection.content
  (#set! injection.language "bash"))

(call
  method: (identifier) @_method (#any-of? @_method "system" "spawn" "exec")
  arguments: (argument_list
    (string
      (string_content) @injection.content))
  (#set! injection.language "bash"))
