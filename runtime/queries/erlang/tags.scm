; Modules
(attribute
  name: (atom) @_attr
  (arguments (atom) @definition.module)
 (#eq? @_attr "module"))

; Constants
((attribute
    name: (atom) @_attr
    (arguments
      .
      [
        (atom) @definition.constant
        (call function: [(variable) (atom)] @definition.macro)
      ]))
 (#eq? @_attr "define"))

; Record definitions
((attribute
   name: (atom) @_attr
   (arguments
     .
     (atom) @definition.struct))
 (#eq? @_attr "record"))

; Function specs
((attribute
    name: (atom) @_attr
    (stab_clause name: (atom) @definition.interface))
 (#eq? @_attr "spec"))

; Types
((attribute
    name: (atom) @_attr
    (arguments
      (binary_operator
        left: [
          (atom) @definition.type
          (call function: (atom) @definition.type)
        ]
        operator: "::")))
 (#any-of? @_attr "type" "opaque"))

; Functions
(function_clause name: (atom) @definition.function)
