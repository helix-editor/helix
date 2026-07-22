; Functions
(private_function
  (function_signature
    (identifier) @definition.function))

(read_only_function
  (function_signature
    (identifier) @definition.function))

(public_function
  (function_signature
    (identifier) @definition.function))

; Traits
(trait_definition
  "define-trait" . (identifier) @definition.interface)

; Constants
(constant_definition
  "define-constant" . (identifier) @definition.constant)

; Data variables
(variable_definition
  "define-data-var" . (identifier) @definition.constant)

; Maps
(mapping_definition
  "define-map" . (identifier) @definition.struct)

; Tokens
(fungible_token_definition
  "define-fungible-token" . (identifier) @definition.type)

(non_fungible_token_definition
  "define-non-fungible-token" . (identifier) @definition.type)
