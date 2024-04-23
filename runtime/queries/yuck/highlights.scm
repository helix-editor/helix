; Errors

(ERROR) @error

; Comments

(comment) @comment

; Operators

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "||"
  "&&"
  "=="
  "!="
  "=~"
  ">"
  "<"
  ">="
  "<="
  "!"
  "?."
  "?:"
] @operator

(ternary_expression
  ["?" ":"] @operator)

; Punctuation

[ ":" "." "," ] @punctuation.delimiter

[ "{" "}" "[" "]" "(" ")" ] @punctuation.bracket

; Literals

(number (float)) @constant.numeric.float

(number (integer)) @constant.numeric.integer

(boolean) @constant.builtin.boolean

; Strings

(escape_sequence) @constant.character.escape

(string_interpolation
  "${" @punctuation.special
  "}" @punctuation.special)

[ (string_fragment) "\"" "'" "`" ] @string

; Attributes & Fields

(keyword) @attribute

; Functions

(function_call
  name: (ident) @function)

; Variables

(ident) @variable

(array
  (symbol) @variable)

; Builtin widgets

(list .
  ((symbol) @tag.builtin
    (#match? @tag.builtin "^(box|button|calendar|centerbox|checkbox|circular-progress|color-button|color-chooser|combo-box-text|eventbox|expander|graph|image|input|label|literal|overlay|progress|revealer|scale|scroll|transform)$")))

; Keywords

; I think there's a bug in tree-sitter the anchor doesn't seem to be working, see
; https://github.com/tree-sitter/tree-sitter/pull/2107
(list .
  ((symbol) @keyword
    (#match? @keyword "^(defwindow|defwidget|defvar|defpoll|deflisten|geometry|children|struts)$")))

(list .
  ((symbol) @keyword.control.import
    (#eq? @keyword.control.import "include")))

; Loop

(loop_widget . "for" @keyword.control.repeat . (symbol) @variable . "in" @keyword.operator . (symbol) @variable)

(loop_widget . "for" @keyword.control.repeat . (symbol) @variable . "in" @keyword.operator)

; Tags

; TODO apply to every symbol in list? I think it should probably only be applied to the first child of the list
(list
  (symbol) @tag)

; Other stuff that has not been caught by the previous queries yet

(ident) @variable
(index) @variable
