; Modules
((attribute
  name: (atom) @_attr
  (arguments (atom) @name)) @definition.module
 (#eq? @_attr "module"))

; Constants
((attribute
    name: (atom) @_attr
    (arguments . (atom) @name)) @definition.constant
 (#eq? @_attr "define"))

; Macros (with arguments)
((attribute
    name: (atom) @_attr
    (arguments . (call function: [(variable) (atom)] @name))) @definition.macro
 (#eq? @_attr "define"))

; Record definitions
((attribute
   name: (atom) @_attr
   (arguments
     .
     (atom) @name)) @definition.struct
 (#eq? @_attr "record"))

((attribute
  name: (atom) @_attr
  (arguments
    .
    [(atom) (macro)] ; Record name
    [
      ; Just the field name:
      (tuple (atom)? @name)
      ; Field name, type OR default:
      (tuple
        (binary_operator
          left: (atom) @name
          operator: ["=" "::"]))
      ; Field name, type AND default:
      (tuple
        (binary_operator
          left:
            (binary_operator
              left: (atom) @name
              operator: "=")
          operator: "::"))
    ])) @definition.field
 (#eq? @_attr "record"))

; Function specs
((attribute
    name: (atom) @_attr
    (stab_clause name: (atom) @name)) @definition.interface
 (#any-of? @_attr "spec" "callback"))

; Types
((attribute
    name: (atom) @_attr
    (arguments
      (binary_operator
        left: [
          (atom) @name
          (call function: (atom) @name)
        ]
        operator: "::"))) @definition.type
 (#any-of? @_attr "type" "opaque"))

; Functions
(function_clause name: (atom) @name) @definition.function
