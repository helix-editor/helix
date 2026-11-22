(block) @local.scope

(command
  (known_command
    (declare_command) @_dc
    (#any-of? @_dc "global" "set"))
  (arguments
    (argument
      (tag))*
    .
    (argument
      (string
        (_
          (string_content) @local.definition.variable.mutable))
    )
  )
)

(variable_interpolation (variable_name) @local.reference)
