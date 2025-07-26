;; From nvim-treesitter, contributed by @ObserverOfTime et al.

; Variables
(variable) @variable

((variable) @constant.builtin
  (#eq? @constant.builtin "$ENV"))

((variable) @constant.builtin
  (#eq? @constant.builtin "$__loc__"))

; Properties
(index
  (identifier) @variable.other.member)

; Labels
(query
  label: (variable) @label)

(query
  break_statement: (variable) @label)

; Literals
(number) @constant.numeric

(string) @string

[
  "true"
  "false"
] @constant.builtin.boolean

"null" @type.builtin

; Interpolation
[
  "\\("
  ")"
] @special

; Format
(format) @attribute

; Functions
(funcdef
  (identifier) @function)

(funcdefargs
  (identifier) @variable.parameter)

[
  "reduce"
  "foreach"
] @function.builtin

((funcname) @function
  .
  "(")

; jq -n 'builtins | map(split("/")[0]) | unique | .[]'
((funcname) @function.builtin
  (#any-of? @function.builtin
    "IN" "INDEX" "JOIN" "abs" "acos" "acosh" "add" "all" "any" "arrays" "ascii_downcase"
    "ascii_upcase" "asin" "asinh" "atan" "atan2" "atanh" "booleans" "bsearch" "builtins" "capture"
    "cbrt" "ceil" "combinations" "contains" "copysign" "cos" "cosh" "debug" "del" "delpaths" "drem"
    "empty" "endswith" "env" "erf" "erfc" "error" "exp" "exp10" "exp2" "explode" "expm1" "fabs"
    "fdim" "finites" "first" "flatten" "floor" "fma" "fmax" "fmin" "fmod" "format" "frexp"
    "from_entries" "fromdate" "fromdateiso8601" "fromjson" "fromstream" "gamma" "get_jq_origin"
    "get_prog_origin" "get_search_list" "getpath" "gmtime" "group_by" "gsub" "halt" "halt_error"
    "has" "hypot" "implode" "in" "index" "indices" "infinite" "input" "input_filename"
    "input_line_number" "inputs" "inside" "isempty" "isfinite" "isinfinite" "isnan" "isnormal"
    "iterables" "j0" "j1" "jn" "join" "keys" "keys_unsorted" "last" "ldexp" "length" "lgamma"
    "lgamma_r" "limit" "localtime" "log" "log10" "log1p" "log2" "logb" "ltrimstr" "map" "map_values"
    "match" "max" "max_by" "min" "min_by" "mktime" "modf" "modulemeta" "nan" "nearbyint" "nextafter"
    "nexttoward" "normals" "not" "now" "nth" "nulls" "numbers" "objects" "path" "paths" "pick" "pow"
    "pow10" "range" "recurse" "remainder" "repeat" "reverse" "rindex" "rint" "round" "rtrimstr"
    "scalars" "scalb" "scalbln" "scan" "select" "setpath" "significand" "sin" "sinh" "sort"
    "sort_by" "split" "splits" "sqrt" "startswith" "stderr" "strflocaltime" "strftime" "strings"
    "strptime" "sub" "tan" "tanh" "test" "tgamma" "to_entries" "todate" "todateiso8601" "tojson"
    "tonumber" "tostream" "tostring" "transpose" "trunc" "truncate_stream" "type" "unique"
    "unique_by" "until" "utf8bytelength" "values" "walk" "while" "with_entries" "y0" "y1" "yn"))

; Keywords
[
  "def"
  "as"
  "label"
  "module"
  "break"
] @keyword

[
  "import"
  "include"
] @keyword.control.import

[
  "if"
  "then"
  "elif"
  "else"
  "end"
] @keyword.control.conditional

[
  "try"
  "catch"
] @keyword.control.exception

[
  "or"
  "and"
] @keyword.operator

; Operators
[
  "."
  "=="
  "!="
  ">"
  ">="
  "<="
  "<"
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "+="
  "-="
  "*="
  "/="
  "%="
  "//="
  "|"
  "?"
  "//"
  "?//"
  (recurse) ; ".."
] @operator

; Punctuation
[
  ";"
  ","
  ":"
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
  "("
  ")"
] @punctuation.bracket

; Comments
(comment) @comment.line
