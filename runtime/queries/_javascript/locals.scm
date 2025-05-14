; Definitions
;------------
; Javascript and Typescript Treesitter grammars deviate when defining the
; tree structure for parameters, so we need to address them in each specific
; language instead of ecma.

; (i)
(formal_parameters 
  (identifier) @local.definition.variable.parameter)

; (i = 1)
(formal_parameters 
  (assignment_pattern
    left: (identifier) @local.definition.variable.parameter))
