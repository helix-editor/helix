[ "(" ")" "[" "]" "{" "}" ] @rainbow.bracket

[
  ; Functions and main block
  (function_definition)
  (main_block)

  ; Control flow blocks
  (if_cond)
  (if_chain)
  (if_ternary)
  (for_loop)
  (while_loop)
  (loop_infinite)

  ; General blocks
  (block)
  (command_modifier_block)

  ; Collections and grouping
  (array)
  (parameter_list)
  (function_parameter_list)
  (parentheses)

  ; Handlers
  (handler_failed)
  (handler_succeeded)
  (handler_exited)
] @rainbow.scope
