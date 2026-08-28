((comment) @injection.content
 (#set! injection.language "comment"))

(command
  name: (command_name (word) @_command)
  argument: (raw_string) @injection.content
 (#match? @_command "^[gnm]?awk$")
 (#set! injection.language "awk"))

((regex) @injection.content
  (#set! injection.language "regex"))

(command
  name: (command_name (word) @_command (#any-of? @_command "jq" "jaq"))
  argument: [
    (raw_string) @injection.content
    (string (string_content) @injection.content)
  ]
  (#set! injection.language "jq"))

(command
  name: (command_name (word) @_command (#eq? @_command "alias"))
  argument: (concatenation
    (word)
    [
      (raw_string) @injection.content
      (string (string_content) @injection.content)
    ])
  (#set! injection.language "bash"))

(command
  name: (command_name (word) @_command (#any-of? @_command "eval" "trap"))
  .
  argument: [
    (raw_string) @injection.content
    (string (string_content) @injection.content)
  ]
  (#set! injection.language "bash"))
