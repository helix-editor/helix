((comment) @injection.content
  (#set! injection.language "comment"))

(command
  head: ((cmd_identifier) @_cmd
    (#match? @_cmd "^\\s*(find|parse|split|str)$"))
  flag: (_
    name: (_) @_flag
    (#any-of? @_flag "r" "regex"))
  .
  arg: (_
    (string_content) @injection.content
    (#set! injection.language "regex")))

(_
  opr: [
    "=~"
    "!~"
    "like"
    "not-like"
  ]
  rhs: (_
    (string_content) @injection.content
    (#set! injection.language "regex")))

(command
  head: (_) @_cmd
  (#any-of? @_cmd "nu" "$nu.current-exe")
  flag: (_
    name: (_) @_flag
    (#any-of? @_flag "c" "e" "commands" "execute"))
  .
  arg: (_
    (string_content) @injection.content
    (#set! injection.language "nu")))

(command
  head: (cmd_identifier) @_command (#any-of? @_command "jq" "jaq")
  .
  arg: (_ (string_content) @injection.content)
  (#set! injection.language "jq")
)

(command
  head: (cmd_identifier) @_command (#eq? @_command "fish")
  flag: (short_flag "-") @_flag (#match? @_flag "^-.*c$")
  .
  arg: (_ (string_content) @injection.content)
  (#set! injection.language "fish")
)
