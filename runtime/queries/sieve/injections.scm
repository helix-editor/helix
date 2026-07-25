; Specify nested languages that live within a `siv` file

; ================ Always applicable ================

((comment_line) @injection.content
  (#set! injection.language "comment"))

((comment_block) @injection.content
  (#set! injection.language "comment"))

; ================ Global defaults ================

; `:matches` are technically globs and not regexes, but it's fine
(arguments
  (argument
    ((tag) @_tag (#any-of? @_tag ":matches" ":regex"))
  )
  .
  (argument)
  .
  (argument
    ((string) @injection.content
      (#set! injection.language "regex")
      (#set! injection.include-children))
  )
)
