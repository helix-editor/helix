; Block structures that increase indent
[
  (function_definition)
  (block)
  (main_block)

  ; Control flow
  (if_cond)
  (if_chain)
  (for_loop)
  (while_loop)
  (loop_infinite)

  ; Collections
  (array)
  (parameter_list)
  (function_parameter_list)
  (parentheses)

  ; Amber-specific
  (command_modifier_block)
  (handler_failed)
  (handler_succeeded)
  (handler_exited)
] @indent

; Closing delimiters
[
  "}"
  "]"
  ")"
] @outdent

; Multi-line construct support
[
  (function_definition)
  (if_cond)
  (if_chain)
  (for_loop)
  (while_loop)
  (loop_infinite)
  (main_block)
] @extend

; Prevent premature outdent
[
  (function_control_flow)
  (loop_control_flow)
] @extend.prevent-once
