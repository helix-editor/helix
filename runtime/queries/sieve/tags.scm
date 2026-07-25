(command
  (known_command
    (action_command) @_command
    (#eq? @_command "include"))
  (arguments
    (argument
      (tag))*
    .
    (argument
      (string
        (_
          (string_content) @name))
    )
  )
) @definition.module
