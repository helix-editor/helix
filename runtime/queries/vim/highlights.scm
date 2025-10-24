(identifier) @variable

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))

; Keywords
[
  "if"
  "else"
  "elseif"
  "endif"
] @keyword.control.conditional

[
  "try"
  "catch"
  "finally"
  "endtry"
  "throw"
] @keyword.control.except

[
  "for"
  "endfor"
  "in"
  "while"
  "endwhile"
  "break"
  "continue"
] @keyword.control.repeat

[
  "function"
  "endfunction"
] @keyword.function

; Function related
(function_declaration
  name: (_) @function)

(call_expression
  function: (identifier) @function)

(call_expression
  function:
    (scoped_identifier
      (identifier) @function))

(parameters
  (identifier) @variable.parameter)

(default_parameter
  (identifier) @variable.parameter)

[
  (bang)
  (spread)
] @punctuation.special

[
  (no_option)
  (inv_option)
  (default_option)
  (option_name)
] @variable.builtin

[
  (scope)
  "a:"
  "$"
] @namespace

; Commands and user defined commands
[
  "let"
  "unlet"
  "const"
  "call"
  "execute"
  "normal"
  "set"
  "setfiletype"
  "setlocal"
  "silent"
  "echo"
  "echon"
  "echohl"
  "echomsg"
  "echoerr"
  "autocmd"
  "augroup"
  "return"
  "syntax"
  "filetype"
  "source"
  "lua"
  "ruby"
  "perl"
  "python"
  "highlight"
  "command"
  "delcommand"
  "comclear"
  "colorscheme"
  "scriptencoding"
  "startinsert"
  "stopinsert"
  "global"
  "runtime"
  "wincmd"
  "cnext"
  "cprevious"
  "cNext"
  "vertical"
  "leftabove"
  "aboveleft"
  "rightbelow"
  "belowright"
  "topleft"
  "botright"
  (unknown_command_name)
  "edit"
  "enew"
  "find"
  "ex"
  "visual"
  "view"
  "eval"
  "sign"
] @keyword

(map_statement
  cmd: _ @keyword)

(keycode) @constant.character.escape

(command_name) @function.macro

; Filetype command
(filetype_statement
  [
    "detect"
    "plugin"
    "indent"
    "on"
    "off"
  ] @keyword)

; Syntax command
(syntax_statement
  (keyword) @string)

(syntax_statement
  [
    "enable"
    "on"
    "off"
    "reset"
    "case"
    "spell"
    "foldlevel"
    "iskeyword"
    "keyword"
    "match"
    "cluster"
    "region"
    "clear"
    "include"
  ] @keyword)

(syntax_argument
  name: _ @keyword)

[
  "<buffer>"
  "<nowait>"
  "<silent>"
  "<script>"
  "<expr>"
  "<unique>"
] @constant.builtin

(augroup_name) @namespace

(au_event) @constant

(normal_statement
  (commands) @constant)

; Highlight command
(hl_attribute
  key: _ @variable.parameter
  val: _ @constant)

(hl_group) @type

(highlight_statement
  [
    "default"
    "link"
    "clear"
  ] @keyword)

; Command command
(command) @string

(command_attribute
  name: _ @variable.parameter)

(command_attribute
  val: (behavior
         _ @constant))

; Edit command
(plus_plus_opt
  val: _? @constant) @variable.parameter

(plus_cmd
  "+" @variable.parameter) @variable.parameter

; Runtime command
(runtime_statement
  (where) @keyword.operator)

; Colorscheme command
(colorscheme_statement
  (name) @string)

; Scriptencoding command
(scriptencoding_statement
  (encoding) @string.special)

; Literals
(string_literal) @string

(integer_literal) @constant.numeric.integer

(float_literal) @constant.numeric.float

(comment) @comment

(line_continuation_comment) @comment

(pattern) @string.special

(pattern_multi) @string.regexp

(filename) @string.special.path

(heredoc
  (body) @string)

(heredoc
  (parameter) @keyword)

[
  (marker_definition)
  (endmarker)
] @label

(literal_dictionary
  (literal_key) @variable.parameter)

((scoped_identifier
  (scope) @_scope
  .
  (identifier) @constant.builtin.boolean)
  (#eq? @_scope "v:")
  (#any-of? @constant.builtin.boolean "true" "false"))

; Operators
[
  "||"
  "&&"
  "&"
  "+"
  "-"
  "*"
  "/"
  "%"
  ".."
  "is"
  "isnot"
  "=="
  "!="
  ">"
  ">="
  "<"
  "<="
  "=~"
  "!~"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  ".="
  "..="
  "<<"
  "=<<"
  (match_case)
] @operator

; Some characters have different meanings based on the context
(unary_operation
  "!" @operator)

(binary_operation
  "." @operator)

; Punctuation
[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
  "#{"
] @punctuation.bracket

(field_expression
  "." @punctuation.delimiter)

[
  ","
  ":"
] @punctuation.delimiter

(ternary_expression
  [
    "?"
    ":"
  ] @keyword.operator)

; Options
((set_value) @constant.numeric
  (#match? @constant.numeric "^[0-9]+([.][0-9]+)?$"))

(inv_option
  "!" @operator)

(set_item
  "?" @operator)

((set_item
  option: (option_name) @_option
  value: (set_value) @function)
  (#any-of? @_option "tagfunc" "tfu" "completefunc" "cfu" "omnifunc" "ofu" "operatorfunc" "opfunc"))
