; Line and block comments use the "comment" pseudo-language for spell-checking
; and URL highlighting in editors that support it.
((line_comment) @injection.content
  (#set! injection.language "comment"))

((block_comment) @injection.content
  (#set! injection.language "comment"))

; (cmd "shell string") injects Bash into the string argument.
; Gives full Bash highlighting inside any cmd action string.
((list
    head: (identifier) @_name
    body: (string) @injection.content)
  (#eq? @_name "cmd")
  (#set! injection.language "bash"))

((list
    head: (identifier) @_name
    body: (string) @injection.content)
  (#eq? @_name "cmd-log")
  (#set! injection.language "bash"))
