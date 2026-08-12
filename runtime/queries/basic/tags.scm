; Old line-numbered BASIC has no functions/types; tag the variables it
; introduces. There is no `variable` tag kind, so use the closest recognized
; one (`constant`)
(for_statement
  variable: (identifier) @name) @definition.constant

(let_statement
  variable: (identifier) @name) @definition.constant
