((comment) @injection.content
 (#set! injection.language "comment"))

(command
  name: (command_name (word) @_command)
  argument: (raw_string) @injection.content
 (#match? @_command "^[gnm]?awk$")
 (#set! injection.language "awk"))