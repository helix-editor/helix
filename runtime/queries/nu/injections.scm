((comment) @injection.content
 (#set! injection.language "comment"))

(command
  head: (cmd_identifier) @_command (#any-of? @_command "jq" "jaq")
  arg: (val_string) @injection.content
  (#set! injection.language "jq"))
