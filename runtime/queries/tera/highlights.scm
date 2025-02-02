; Namespaces

(import_statement
  scope: (identifier) @namespace)

; Functions

(filter_expression
  filter: (identifier) @function.builtin
  (#any-of? @function.builtin
    ; Filters - https://keats.github.io/tera/docs/#built-in-filters
    "lower"
    "upper"
    "wordcount"
    "capitalize"
    "replace"
    "addslashes"
    "slugify"
    "title"
    "trim"
    "trim_start"
    "trim_end"
    "trim_start_matches"
    "trim_end_matches"
    "truncate"
    "linebreaksbr"
    "spaceless"
    "indent"
    "striptags"
    "first"
    "last"
    "nth"
    "join"
    "length"
    "reverse"
    "sort"
    "unique"
    "slice"
    "group_by"
    "filter"
    "map"
    "concat"
    "urlencode"
    "urlencode_strict"
    "abs"
    "pluralize"
    "round"
    "filesizeformat"
    "date"
    "escape"
    "escape_xml"
    "safe"
    "get"
    "split"
    "int"
    "float"
    "json_encode"
    "as_str"
    "default"))

(filter_expression
  filter: (identifier) @function.method)

(test_expression
  test: (identifier) @function.builtin
  (#any-of? @function.builtin
    ; Tests - https://keats.github.io/tera/docs/#built-in-tests
    "defined"
    "undefined"
    "odd"
    "even"
    "string"
    "number"
    "divisibleby"
    "iterable"
    "object"
    "starting_with"
    "ending_with"
    "containing"
    "matching"))

(test_expression
  test: (identifier) @function)

(call_expression
  name: (identifier) @function.builtin
  (#any-of? @function.builtin
    ; Functions - https://keats.github.io/tera/docs/#built-in-functions
    "range"
    "now"
    "throw"
    "get_random"
    "get_env"))

(call_expression
  scope: (identifier)? @namespace
  name: (identifier) @function)

(macro_statement
  name: (identifier) @function
  (parameter_list
    parameter: (identifier) @variable.parameter
    (optional_parameter
      name: (identifier) @variable.parameter)))

; Keywords

[
  "set"
  "set_global"
  "filter"
  "endfilter"
  "block"
  "endblock"
  "macro"
  "endmacro"
  "raw"
  "endraw"
  "as"
] @keyword

[
  "break"
  "continue"
] @keyword.control.return

[
  "in"
  "and"
  "or"
  "not"
  "is"
] @keyword.operator

[
  "include"
  "import"
  "extends"
] @keyword.control.import

[
  "for"
  "endfor"
] @keyword.control.repeat

[
  "if"
  "elif"
  "else"
  "endif"
] @keyword.control.conditional

; Tags

(comment_tag) @comment

; Tokens

[
  "("
  ")"
  "["
  "]"
  "{%"
  "%}"
  "-%}"
  "{%-"
  "}}"
  "{{"
  "-}}"
  "{{-"
  "::"
] @punctuation.bracket

[
  "*"
  "/"
  "%"
  "|"
  "+"
  "-"
  "~"
  "="
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
] @operator

[
  "."
  ","
] @punctuation.delimiter


; Literals

(number) @constant.numeric

(bool) @constant.builtin

(string) @string

; Properties

(member_expression
  property: (identifier)? @variable.other.member)

; Variables

((identifier) @variable.builtin
  (#any-of? @variable.builtin
    "loop"
    "__tera_context"))

(identifier) @variable
