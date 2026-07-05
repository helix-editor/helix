((comment) @injection.content
  (#set! injection.language "comment"))

; e.g. `echo hello123 | findstr /R "[a-z]*[0-9][0-9][0-9]"`
(cmd
  (command_name) @_command
  (argument_list
    (string) @injection.content)
  (#eq? @_command "findstr")
  (#set! injection.language "regex"))

