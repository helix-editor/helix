; Most primitive nodes
(shebang) @keyword.directive

[
  (symbol)
  (symbol_binding)
] @variable

(comment) @comment

(fn_form
  name: [
    (symbol) @function
    (multi_symbol
      member: (symbol_fragment) @function .)
  ])

(lambda_form
  name: [
    (symbol) @function
    (multi_symbol
      member: (symbol_fragment) @function .)
  ])

((symbol) @operator
  (#any-of? @operator
    ; arithmetic
    "+" "-" "*" "/" "//" "%" "^"
    ; comparison
    ">" "<" ">=" "<=" "=" "~="
    ; other
    "#" "." "?." ".."))

((symbol) @keyword.operator
  (#any-of? @keyword.operator
    ; comparison
    "not="
    ; boolean
    "and" "or" "not"
    ; bitwise
    "lshift" "rshift" "band" "bor" "bxor" "bnot"
    ; other
    "length"))

(case_guard
  call: (_) @keyword.control.conditional)

(case_guard_or_special
  call: (_) @keyword.control.conditional)

(case_catch
  call: (symbol) @keyword)

(import_macros_form
  imports: (table_binding
    (table_binding_pair
      value: (symbol_binding) @function.macro)))


((symbol) @keyword.function
  (#any-of? @keyword.function "fn" "lambda" "λ" "hashfn"))

((symbol) @keyword.control.repeat
  (#any-of? @keyword.control.repeat "for" "each" "while"))

((symbol) @keyword.control.conditional
  (#any-of? @keyword.control.conditional "if" "when" "match" "case" "match-try" "case-try"))

((symbol) @keyword
  (#any-of? @keyword
    "global" "local" "let" "set" "var" "comment" "do" "doc" "eval-compiler" "lua" "macros" "unquote"
    "quote" "tset" "values" "tail!"))

((symbol) @keyword.control.import
  (#any-of? @keyword.control.import "require" "require-macros" "import-macros" "include"))

((symbol) @function.macro
  (#any-of? @function.macro
    "collect" "icollect" "fcollect" "accumulate" "faccumulate" "->" "->>" "-?>" "-?>>" "?." "doto"
    "macro" "macrodebug" "partial" "pick-args" "pick-values" "with-open"))

((symbol) @variable.builtin
  (#eq? @variable.builtin "..."))

((symbol) @constant.builtin
  (#eq? @constant.builtin "_VERSION"))

((symbol) @function.builtin
  (#any-of? @function.builtin
    "assert" "collectgarbage" "dofile" "error" "getmetatable" "ipairs" "load" "loadfile" "next"
    "pairs" "pcall" "print" "rawequal" "rawget" "rawlen" "rawset" "require" "select" "setmetatable"
    "tonumber" "tostring" "type" "warn" "xpcall" "module" "setfenv" "loadstring" "unpack"))

; TODO: Highlight builtin methods (`table.unpack`, etc) as @function.builtin
([
  (symbol) @variable.builtin
  (multi_symbol
    base: (symbol_fragment) @variable.builtin)
]
  (#any-of? @variable.builtin
    "vim" "_G" "_ENV" "debug" "io" "jit" "math" "os" "package" "string" "table" "utf8"))

([
  (symbol) @variable.builtin
  (multi_symbol
    .
    (symbol_fragment) @variable.builtin)
]
  (#eq? @variable.builtin "arg"))
(symbol_option) @keyword.directive

(escape_sequence) @constant.character.escape

(multi_symbol
  "." @punctuation.delimiter
  member: (symbol_fragment) @variable.other.member)

(list
  call: (symbol) @function)

(list
  call: (multi_symbol
    member: (symbol_fragment) @function .))

(multi_symbol_method
  ":" @punctuation.delimiter
  method: (symbol_fragment) @function.method)

(quasi_quote_reader_macro
  macro: _ @punctuation.special)

(quote_reader_macro
  macro: _ @punctuation.special)

(unquote_reader_macro
  macro: _ @punctuation.special)

(hashfn_reader_macro
  macro: _ @keyword.function)

(docstring) @comment.block.documentation

; NOTE: The macro name is highlighted as @variable because it
; gives a nicer contrast instead of everything being the same
; color. Rust queries use this workaround too for `macro_rules!`.
(macro_form
  name: [
    (symbol) @variable
    (multi_symbol
      member: (symbol_fragment) @variable .)
  ])

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

(sequence_arguments
  (symbol_binding) @variable.parameter)

(sequence_arguments
  (rest_binding
    rhs: (symbol_binding) @variable.parameter))

((symbol) @variable.parameter
  (#any-of? @variable.parameter "$" "$..."))

((symbol) @variable.parameter
  (#any-of? @variable.parameter "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9"))

[
  (nil)
  (nil_binding)
] @constant.builtin

[
  (boolean)
  (boolean_binding)
] @constant.builtin.boolean

[
  (number)
  (number_binding)
] @constant.numeric

[
  (string)
  (string_binding)
] @string
