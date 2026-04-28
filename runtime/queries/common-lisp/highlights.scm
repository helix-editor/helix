(sym_lit) @variable

[
  (accumulation_verb)
  "thereis"
  "always"
  "below"
  "into"
  "as"
] @keyword

"and" @keyword.operator

[
  "when"
  "if"
  "unless"
  "else"
] @keyword.control.conditional

[
  "for"
  "loop"
  "while"
  "until"
  "do"
  "repeat"
] @keyword.control.loop

[
  "in"
  "across"
  "being"
  "from"

  "finally"
  "initially"

  "with"
] @keyword.control

"return" @keyword.control.return

(include_reader_macro) @keyword.directive

(defun_keyword) @keyword.function

(defun_header
  function_name: (_) @function)

(defun_header
  lambda_list: (list_lit
    (sym_lit) @variable.parameter))

(defun_header
  lambda_list: (list_lit
    (list_lit
      . (sym_lit) @variable.parameter)
      . (_)))

"=" @operator

; quote
(format_specifier) @operator

(quoting_lit "'" @operator)
(syn_quoting_lit "`" @operator)
(unquoting_lit "," @operator)
(unquote_splicing_lit ",@" @operator)

(var_quoting_lit
  marker: "#'" @operator)

(list_lit
  . (sym_lit) @operator
  (#any-of? @operator "+" "*" "-" "=" "<" ">" "<=" ">=" "/="))

(package_lit
  package: (_) @namespace)
"cl" @namespace

(str_lit) @string

(num_lit) @constant.numeric
["#c" "#C"] @constant.numeric
[
  (array_dimension)
  "#0A"
  "#0a"
] @constant.numeric

(nil_lit) @constant.builtin
(char_lit) @constant.character

[(comment) (block_comment)] @comment
(dis_expr) @comment

["(" ")"] @punctuaton.bracket
[":" "::" "."] @punctuation.special
