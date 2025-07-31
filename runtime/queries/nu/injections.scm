((comment) @injection.content
 (#set! injection.language "comment"))

; (find|parse) --regex <pattern>
; str replace (-r|--regex)
; split (column|list|row) (-r|--regex) <pattern>
(command
  flag: (_) @_flag (#any-of? @_flag "-r" "--regex")
  .
  arg: (val_string) @injection.content
  (#set! injection.language "regex"))

; polars str-replace(-all)? (-p|--pattern) <pattern>
(command
  head: (cmd_identifier) @_cmd (#eq? @_cmd "polars")
  arg_str: (val_string) @_subcmd (#any-of? @_subcmd "str-replace" "str-replace-all")
  flag: (_) @_flag (#any-of? @_flag "-p" "--pattern")
  .
  arg: (val_string) @injection.content
  (#set! injection.language "regex"))

; polars contains <pattern>
(command
  head: (cmd_identifier) @_cmd (#eq? @_cmd "polars")
  arg_str: (val_string) @_subcmd (#eq? @_subcmd "contains")
  .
  arg: (val_string) @injection.content
  (#set! injection.language "regex"))

(expr_binary
  ["=~" "!~"]
  rhs: (val_string) @injection.content
  (#set! injection.language "regex"))
