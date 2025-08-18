((comment) @injection.content
 (#set! injection.language "comment"))

(command
  name: (word) @_command (#any-of? @_command "jq" "jaq")
  argument: [(double_quote_string) (single_quote_string)] @injection.content
  (#set! injection.language "jq"))
