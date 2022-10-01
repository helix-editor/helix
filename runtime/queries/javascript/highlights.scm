; Function and method parameters
;-------------------------------

; (p) => ...
(formal_parameters 
  (identifier) @variable.parameter)

; (...p) => ...
(formal_parameters
  (rest_pattern
    (identifier) @variable.parameter))

; ({ p }) => ...
(formal_parameters
  (object_pattern
    (shorthand_property_identifier_pattern) @variable.parameter))

; ({ a: p }) => ...
(formal_parameters
  (object_pattern
    (pair_pattern
      value: (identifier) @variable.parameter)))

; ([ p ]) => ...
(formal_parameters
  (array_pattern
    (identifier) @variable.parameter))

; (p = 1) => ...
(formal_parameters
  (assignment_pattern
    left: (identifier) @variable.parameter))

; p => ...
(arrow_function
  parameter: (identifier) @variable.parameter)

; inherits: ecma
