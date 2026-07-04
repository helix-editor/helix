; Variables
(iden_type
  name: (identifier) @variable)

[
 "this"
 ] @variable.builtin

(named_tuple_access_expr
  (expr
    (primitive_expr
      (identifier)))
  field: (identifier) @variable.other.member)

; Functions
(p_fun_decl
  name: (identifier) @function)

[
 "choose"
 "format"
 "keys"
 "values"
 ] @function.builtin

; Comments
[
 (block_comment)
 (line_comment)
 ] @comment

; Types
[
 "any"
 "bool"
 "eventset"
 "float"
 "int"
 "interface"
 "map"
 "set"
 "string"
 "seq"
 "data"
 ] @type.builtin

; Type definitions
(type_def_decl
  name: (identifier) @type.definition)
(enum_type_def_decl
  name: (identifier) @type.definition)
(event_decl
  name: (identifier) @type.definition)
(impl_machine_decl
  name: (identifier) @type.definition)
(spec_machine_decl
  name: (identifier) @type.definition)
(state_decl
  name: (identifier) @type.definition)

(named_type
  name: (identifier) @type)
(state_name
  state: (identifier) @type)
(enum_elem
  name: (identifier) @type)
(non_default_event
  (identifier) @type)
(event_id
  (identifier) @type)

; Keywords
[
 "announce"
 "as"
 "assert"
 "break"
 "case"
 "continue"
 "default"
 "defer"
 "do"
 "entry"
 "exit"
 "foreach"
 "goto"
 "halt"
 "ignore"
 "in"
 "new"
 "observes"
 "on"
 "print"
 "raise"
 "receive"
 "send"
 "sizeof"
 "spec"
 "start"
 "state"
 "var"
 "param"
 "pairwise"
 "wise"
 "with"
 ; PVerifier keywords
 "invariant"
 "axiom"
 "is"
 "inflight"
 "targets"
 "sent"
 "Proof"
 "prove"
 "using"
 "Lemma"
 "Theorem"
 "except"
 "requires"
 "ensures"
 "forall"
 "exists"
 "init-condition"
 "pure"
 "assume"
 ; module-system-specific keywords
 ; module-test-implementation declarations
 "module"
 "implementation"
 "test"
 "refines"
 ; module constructors
 "compose"
 "union"
 "hidee"
 "hidei"
 "rename"
 "main"
 ; machine annotations
 "receives"
 "sends"
 ; common keywords
 "creates"
 "to"
 (temperature_hot)
 (temperature_cold)
 (null_literal)
 ] @keyword

[ "fun" ] @keyword.function

[
 "enum"
 "event"
 "machine"
 "type"
 ] @keyword.storage.type

[ "while" ] @keyword.control.repeat

[ "return" ] @keyword.control.return

[
 "if"
 "else"
 ] @keyword.control.conditional

; Literals
[ (bool_literal) ] @constant.builtin.boolean

(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float

(string_literal) @string

; Symbols / Operators
[
 "$$"
 "$"
 "!"
 "&&"
 "||"
 "==>"
 "<==>"
 "=="
 "!="
 "<="
 ">="
 "<"
 ">"
 "->"
 "="
 "+="
 "-="
 "+"
 "-"
 "*"
 "/"
 "%"
 ] @operator

[
 ":"
 ";"
 ","
 "."
 ] @punctuation.delimiter

[
 "{"
 "}"
 "["
 "]"
 "("
 ")"
 ] @punctuation.bracket
