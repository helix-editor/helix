; Reserved keywords

["when" "and" "or" "not" "in" "fn" "do" "end" "catch" "rescue" "after" "else"] @keyword

; Operators

; * doc string
(unary_operator
  operator: "@" @comment.doc
  operand: (call
    target: (identifier) @comment.doc.__attribute__
    (arguments
      [
        (string) @comment.doc
        (charlist) @comment.doc
        (sigil
          quoted_start: _ @comment.doc
          quoted_end: _ @comment.doc) @comment.doc
        (boolean) @comment.doc
      ]))
  (#match? @comment.doc.__attribute__ "^(moduledoc|typedoc|doc)$"))

; * module attribute
(unary_operator
  operator: "@" @attribute
  operand: [
    (identifier) @attribute
    (call
      target: (identifier) @attribute)
    (boolean) @attribute
    (nil) @attribute
  ])

; * capture operand
(unary_operator
  operator: "&"
  operand: (integer) @operator)

(operator_identifier) @operator

(unary_operator
  operator: _ @operator)

(binary_operator
  operator: _ @operator)

(dot
  operator: _ @operator)

(stab_clause
  operator: _ @operator)

; Literals

[
  (boolean)
  (nil)
] @constant

[
  (integer)
  (float)
] @number

(alias) @type

(char) @constant

; Quoted content

(interpolation "#{" @punctuation.special "}" @punctuation.special) @embedded

(escape_sequence) @string.escape

[
  (atom)
  (quoted_atom)
  (keyword)
  (quoted_keyword)
] @string.special.symbol

[
  (string)
  (charlist)
] @string

; Note that we explicitly target sigil quoted start/end, so they are not overridden by delimiters

(sigil
  (sigil_name) @__name__
  quoted_start: _ @string
  quoted_end: _ @string
  (#match? @__name__ "^[sS]$")) @string

(sigil
  (sigil_name) @__name__
  quoted_start: _ @string.regex
  quoted_end: _ @string.regex
  (#match? @__name__ "^[rR]$")) @string.regex

(sigil
  (sigil_name) @__name__
  quoted_start: _ @string.special
  quoted_end: _ @string.special) @string.special

; Calls

; * definition keyword
(call
  target: (identifier) @keyword
  (#match? @keyword "^(def|defdelegate|defexception|defguard|defguardp|defimpl|defmacro|defmacrop|defmodule|defn|defnp|defoverridable|defp|defprotocol|defstruct)$"))

; * kernel or special forms keyword
(call
  target: (identifier) @keyword
  (#match? @keyword "^(alias|case|cond|else|for|if|import|quote|raise|receive|require|reraise|super|throw|try|unless|unquote|unquote_splicing|use|with)$"))

; * function call
(call
  target: [
    ; local
    (identifier) @function
    ; remote
    (dot
      right: (identifier) @function)
  ])

; * just identifier in function definition
(call
  target: (identifier) @keyword
  (arguments
    [
      (identifier) @function
      (binary_operator
        left: (identifier) @function
        operator: "when")
    ])
  (#match? @keyword "^(def|defdelegate|defguard|defguardp|defmacro|defmacrop|defn|defnp|defp)$"))

; * pipe into identifier (definition)
(call
  target: (identifier) @keyword
  (arguments
    (binary_operator
      operator: "|>"
      right: (identifier) @variable))
  (#match? @keyword "^(def|defdelegate|defguard|defguardp|defmacro|defmacrop|defn|defnp|defp)$"))

; * pipe into identifier (function call)
(binary_operator
  operator: "|>"
  right: (identifier) @function)

; Identifiers

; * special
(
  (identifier) @constant.builtin
  (#match? @constant.builtin "^(__MODULE__|__DIR__|__ENV__|__CALLER__|__STACKTRACE__)$")
)

; * unused
(
  (identifier) @comment.unused
  (#match? @comment.unused "^_")
)

; * regular
(identifier) @variable

; Comment

(comment) @comment

; Punctuation

[
 "%"
] @punctuation

[
 ","
 ";"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<<"
  ">>"
] @punctuation.bracket
