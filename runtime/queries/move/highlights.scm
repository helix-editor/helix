(ability) @keyword

; ---
; Primitives
; ---

(address_literal) @constant
(bool_literal) @constant.builtin.boolean
(num_literal) @constant.numeric
[
  (hex_string_literal)
  (byte_string_literal)
] @string
; TODO: vector_literal

[
  (line_comment)
  (block_comment)
] @comment

(annotation) @function.macro

(borrow_expression "&" @keyword.storage.modifier.ref)
(borrow_expression "&mut" @keyword.storage.modifier.mut)

(identifier) @variable

(constant_identifier) @constant
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

(function_identifier) @function

(primitive_type) @type.builtin

(struct_identifier) @type
(pack_expression
  access: (module_access
    member: (identifier) @type))
(apply_type
  (module_access
    member: (identifier) @type))
(field_identifier) @variable.other.member

; -------
; Functions
; -------

(call_expression
  access: (module_access
    member: (identifier) @function))

(macro_call_expression
  access: (macro_module_access
    access: (module_access
      member: [(identifier) @function.macro])
    "!" @function.macro))

; -------
; Paths
; -------

(module_identifier) @namespace

; -------
; Operators
; -------

[
  "*"
  "="
  "!"
] @operator
(binary_operator) @operator

; ---
; Punctuation
; ---

[
  "::"
  "."
  ";"
  ","
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "abort"
  ; "acquires"
  "as"
  "break"
  "const"
  "continue"
  "copy"
  "else"
  "false"
  "friend"
  "fun"
  "has"
  "if"
  ; "invariant"
  "let"
  "loop"
  "module"
  "move"
  "native"
  "public"
  "return"
  ; "script"
  "spec"
  "struct"
  "true"
  "use"
  "while"  

  "entry"

  ; "aborts_if"
  ; "aborts_with"
  "address"
  "apply"
  "assume"
  ; "axiom"
  ; "choose"
  "decreases"
  ; "emits"
  "ensures"
  "except"
  ; "forall"
  "global"
  "include"
  "internal"
  "local"
  ; "min"
  ; "modifies"
  "mut"
  "phantom"
  "post"
  "pragma"
  ; "requires"
  ; "Self"
  "schema"
  "succeeds_if"
  "to"
  ; "update"
  "where"
  "with"
] @keyword

