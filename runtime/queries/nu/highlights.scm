; ---
; keywords
[
  "let"
  "mut"
  "const"
] @keyword

[
  "if"
  "else"
  "match"
] @keyword.control.conditional

[
  "loop"
  "while"
] @keyword.control.repeat

"def" @keyword.function

[
  "try"
  "catch"
  "error"
] @keyword.control.exception

[
  "module"
  "use"
] @keyword.control.import

[
  "alias"
  "export-env"
  "export"
  "extern"
] @keyword.storage.modifier

(decl_use
  "use" @keyword.control.import)

(ctrl_for
  "for" @keyword.control.repeat
  "in" @keyword.control.repeat)

; ---
; literals
(val_number) @constant.numeric

(val_duration
  unit: _ @variable.parameter)

(val_filesize
  unit: _ @variable.parameter)

(val_binary
  [
    "0b"
    "0o"
    "0x"
  ] @constant.numeric
  "[" @punctuation.bracket
  digit: [
    "," @punctuation.delimiter
    (hex_digit) @constant.numeric
  ]
  "]" @punctuation.bracket) @constant.numeric

(val_bool) @constant.builtin.boolean

(val_nothing) @constant.builtin

(val_string) @string

arg_str: (val_string) @variable.parameter

file_path: (val_string) @variable.parameter

(val_date) @constant.numeric

(inter_escape_sequence) @constant.character.escape

(escape_sequence) @constant.character.escape

(val_interpolated
  [
    "$\""
    "$\'"
    "\""
    "\'"
  ] @string)

(unescaped_interpolated_content) @string

(escaped_interpolated_content) @string

(expr_interpolated
  [
    "("
    ")"
  ] @variable.parameter)

(raw_string_begin) @punctuation.special

(raw_string_end) @punctuation.special

; ---
; operators
(expr_binary
  opr: _ @operator)

(where_predicate
  opr: _ @operator)

(assignment
  [
    "="
    "+="
    "-="
    "*="
    "/="
    "++="
  ] @operator)

(expr_unary
  [
    "not"
    "-"
  ] @operator)

(val_range
  [
    ".."
    "..="
    "..<"
  ] @operator)

[
  "=>"
  "="
  "|"
] @operator

[
  "o>"
  "out>"
  "e>"
  "err>"
  "e+o>"
  "err+out>"
  "o+e>"
  "out+err>"
  "o>>"
  "out>>"
  "e>>"
  "err>>"
  "e+o>>"
  "err+out>>"
  "o+e>>"
  "out+err>>"
  "e>|"
  "err>|"
  "e+o>|"
  "err+out>|"
  "o+e>|"
  "out+err>|"
] @operator

; ---
; punctuation
[
  ","
  ";"
] @punctuation.special

(param_long_flag
  "--" @punctuation.delimiter)

(long_flag
  "--" @punctuation.delimiter)

(short_flag
  "-" @punctuation.delimiter)

(long_flag
  "=" @punctuation.special)

(short_flag
  "=" @punctuation.special)

(param_short_flag
  "-" @punctuation.delimiter)

(param_rest
  "..." @punctuation.delimiter)

(param_type
  ":" @punctuation.special)

(param_value
  "=" @punctuation.special)

(param_cmd
  "@" @punctuation.special)

(attribute
  "@" @punctuation.special)

(param_opt
  "?" @punctuation.special)

(returns
  "->" @punctuation.special)

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
  "...["
  "...("
  "...{"
] @punctuation.bracket

(val_record
  (record_entry
    ":" @punctuation.delimiter))

key: (identifier) @property

; ---
; identifiers
(param_rest
  name: (_) @variable.parameter)

(param_opt
  name: (_) @variable.parameter)

(parameter
  param_name: (_) @variable.parameter)

(param_cmd
  (cmd_identifier) @string)

(param_long_flag
  (long_flag_identifier) @attribute)

(param_short_flag
  (param_short_flag_identifier) @attribute)

(attribute
  (attribute_identifier) @attribute)

(short_flag
  (short_flag_identifier) @attribute)

(long_flag_identifier) @attribute

(scope_pattern
  (wild_card) @function)

(cmd_identifier) @function

(decl_def . "def"
  (val_string
    (string_content) @function
  )
)

; generated with Nu 0.107.0
; help commands
; | where $it.command_type == built-in and $it.category != core
; | each {$'"($in.name | split row " " | $in.0)"'}
; | uniq
; | str join ' '
(command
  head: (cmd_identifier) @function.builtin
  (#any-of? @function.builtin
    "all" "ansi" "any" "append" "ast" "bits" "bytes" "cal" "cd" "char" "chunk-by" "chunks" "clear" "collect" "columns" "compact" "complete" "config" "cp" "date" "debug" "decode" "default" "detect" "drop" "du" "each" "encode" "enumerate" "every" "exec" "exit" "explain" "explore" "fill" "filter" "find" "first" "flatten" "format" "from" "generate" "get" "glob" "grid" "group-by" "hash" "headers" "histogram" "history" "http" "input" "insert" "inspect" "interleave" "into" "is-empty" "is-not-empty" "is-terminal" "items" "job" "join" "keybindings" "kill" "last" "length" "let-env" "lines" "load-env" "ls" "math" "merge" "metadata" "mkdir" "mktemp" "move" "mv" "nu-check" "nu-highlight" "open" "panic" "par-each" "parse" "path" "plugin" "port" "prepend" "print" "ps" "query" "random" "reduce" "reject" "rename" "reverse" "rm" "roll" "rotate" "run-external" "save" "schema" "select" "seq" "shuffle" "skip" "sleep" "slice" "sort" "sort-by" "split" "start" "stor" "str" "sys" "table" "take" "tee" "term" "timeit" "to" "touch" "transpose" "tutor" "ulimit" "uname" "uniq" "uniq-by" "update" "upsert" "url" "values" "version" "view" "watch" "which" "whoami" "window" "with-env" "wrap" "zip"))

(command
  head: (cmd_identifier) @keyword.control.repeat
  (#any-of? @keyword.control.repeat "break" "continue" "return"))

(command
  head: (cmd_identifier) @keyword
  (#any-of? @keyword "do" "source" "source-env" "hide" "hide-env"))

(command
  head: (cmd_identifier) @keyword
  .
  arg_str: (val_string) @keyword.control.import
  (#any-of? @keyword "overlay" "error"))

(command
  head: (cmd_identifier) @cmd
  arg_str: (val_string) @keyword
  (#eq? @cmd "overlay")
  (#eq? @keyword "as"))

(command
  "^" @punctuation.delimiter
  head: (_) @function)

"where" @function.builtin

(where_predicate
  [
    "?"
    "!"
  ] @punctuation.delimiter)

(path
  [
    "."
    "?"
    "!"
  ]? @punctuation.delimiter) @variable.parameter

(stmt_let
  (identifier) @variable)

(val_variable
  "$"? @punctuation.special
  "...$"? @punctuation.special
  [
    (identifier) @variable
    "in" @special
    "nu" @namespace
    "env" @constant
  ]) @none

(val_cellpath
  "$" @punctuation.special)

(record_entry
  ":" @punctuation.special)

; ---
; types
(flat_type) @type

(list_type
  "list" @type.enum
  [
    "<"
    ">"
  ] @punctuation.bracket)

(collection_type
  [
    "record"
    "table"
  ] @type.enum
  "<" @punctuation.bracket
  key: (_) @variable.parameter
  [
    ","
    ":"
  ] @punctuation.special
  ">" @punctuation.bracket)

(composite_type
  "oneof" @type.enum
  [
    "<"
    ">"
  ] @punctuation.bracket)

[(comment) (shebang)] @comment

((comment)+ @comment.documentation
  .
  (decl_def))

(parameter
  (comment) @comment.documentation)

(command
  head: ((cmd_identifier) @function.builtin
    (#match? @function.builtin "^\\s*(find|parse|split|str)$"))
  flag: (_
    name: (_) @attribute
    (#any-of? @attribute "r" "regex"))
  .
  arg: (_
    (string_content) @string.regexp))

(_
  opr: [
    "=~"
    "!~"
    "like"
    "not-like"
  ]
  rhs: (_
    (string_content) @string.regexp))

(command
  head: ((_) @function
    (#any-of? @function "nu" "$nu.current-exe"))
  flag: (_
    name: (_) @attribute
    (#any-of? @attribute "c" "e" "commands" "execute"))
  .
  arg: (_
    (string_content) @string.code))
