;; fallback for identifiers
(ident) @variable

;; Comments

(comment) @comment

; CryptoVerif-only opaque blocks
(proof) @comment.unused
(def) @comment.unused

;; Types
(type_def
  name: (ident) @type)

(typeid) @type
(typeid
  (ident) @type)

(decl_table
  name: (ident) @type)

;; Function & Macro Definitions

(function_def
  name: (ident) @function)

(function_macro_def
  name: (ident) @function.macro)

(decl_pred
  name: (ident) @function)

(decl_event
  name: (ident) @function)

;; Function Calls

(term_function_call
  name: (ident) @function)
(term_function_call
 (term (ident) @variable.parameter))

(pterm_function_call
  name: (ident) @function)
(pterm_function_call
 (pterm (ident) @variable.parameter))

(decl_process_macro
  name: (ident) @function)
(decl_process_macro
 (typedecl (ident) @variable.parameter))

(gterm_function_call
  name: (ident) @function)
(gterm_function_call
 (gterm (ident) @variable.parameter))

(gformat_function_call
  name: (ident) @function)
(gformat_function_call
 (gformat (ident) @variable.parameter))

(process_function_call
  name: (ident) @function)
(process_function_call
 (pterm (ident) @variable.parameter))

(process_input "in" @function.builtin)
(process_input
 (pterm (ident) @variable.parameter))
(process_input
 (pattern (ident) @variable.parameter))
(process_input
 (pattern (pattern (ident) @variable.parameter)))

(process_output "out" @function.builtin)
(process_output
 (pterm (ident) @variable.parameter))
(process_output
 (pterm (pterm (ident) @variable.parameter)))

;; Literals

(nat) @constant.numeric
(int) @constant.numeric
(string) @string

[
  "true"
  "false"
] @constant.builtin.boolean

"fail" @constant.builtin

(process_final_item
  "0" @constant.builtin)

;; Keywords

[
  "among"
  "axiom"
  "choice"
  "clauses"
  "const"
  "def"
  "elimtrue"
  "else"
  "equation"
  "equivalence"
  "event"
  "expand"
  "fail"
  "forall"
  "foreach"
  "free"
  "fun"
  "get"
  "if"
  "inj-event"
  "insert"
  "lemma"
  "let"
  "letfun"
  "letproba"
  "new"
  "noninterf"
  "noselect"
  "not"
  "nounif"
  "otherwise"
  "param"
  "phase"
  "pred"
  "proba"
  "process"
  "proof"
  "public_vars"
  "putbegin"
  "query"
  "reduc"
  "restriction"
  "secret"
  "select"
  "set"
  "suchthat"
  "sync"
  "table"
  "then"
  "type"
  "weaksecret"
  "yield"
] @keyword

(decl_channel "channel" @keyword) ; "channel" can also be used as type, hence the restriction
(process_let "in" @keyword) ; "in" can is keyword in "let ... in", but builtin function otherwise

;; Operators

[
  "="
  "<>"
  "<="
  ">="
  "<"
  ">"
  "+"
  "-"
  "==>"
  "<-"
  "<-R"
  "->"
  "<->"
  "<=>"
  "||"
  "&&"
  "|"
  "!"
] @keyword.operator

;; Punctuation

[
  "."
  ","
  ";"
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

;; Options

(option) @attribute
(nounifoption) @attribute
