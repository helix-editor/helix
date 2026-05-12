((comment) @injection.content
  (#set! injection.language "comment"))

((line_continuation_comment) @injection.content
  (#set! injection.language "comment"))

((shell_fragment) @injection.content
  (#set! injection.language "bash")
  (#set! injection.include-children))
