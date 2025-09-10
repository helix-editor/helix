((comment) @injection.content
 (#set! injection.language "comment"))

(command
  name: (word) @_command (#any-of? @_command "jq" "jaq")
  argument: [(double_quote_string) (single_quote_string)] @injection.content
  (#set! injection.language "jq"))

(command
  name: (word) @_command (#eq? @_command "nu")
  argument: (word) @_flag (#match? @_flag "^-.*c$")
  argument: [(single_quote_string) (double_quote_string)] @injection.content
  (#set! injection.language "nu")
)
