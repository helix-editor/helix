; Function definitions
(function_definition
  name: (variable) @name) @definition.function

; Function calls
(function_call
  name: (variable) @name) @reference.call

; Main block as entry point
(main_block) @definition.function
