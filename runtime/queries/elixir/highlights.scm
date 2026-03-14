; The following code originates mostly from
; https://github.com/elixir-lang/tree-sitter-elixir, with minor edits to
; align the captures with helix. The following should be considered
; Copyright 2021 The Elixir Team
;
; Licensed under the Apache License, Version 2.0 (the "License");
; you may not use this file except in compliance with the License.
; You may obtain a copy of the License at
;
;    https://www.apache.org/licenses/LICENSE-2.0
;
; Unless required by applicable law or agreed to in writing, software
; distributed under the License is distributed on an "AS IS" BASIS,
; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
; See the License for the specific language governing permissions and
; limitations under the License.

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

; Literals

(boolean) @constant.builtin.boolean
(nil) @constant.builtin
(integer) @constant.numeric.integer
(float) @constant.numeric.float
(char) @constant.character

; Identifiers

; * regular
(identifier) @variable

; * unused
(
  (identifier) @comment.unused
  (#match? @comment.unused "^_")
)

; * special
(
  (identifier) @constant.builtin
  (#any-of? @constant.builtin "__MODULE__" "__DIR__" "__ENV__" "__CALLER__" "__STACKTRACE__")
)

; Comment

(comment) @comment

; Quoted content

(interpolation "#{" @punctuation.special "}" @punctuation.special) @embedded

(escape_sequence) @constant.character.escape

[
  (string)
  (charlist)
] @string

[
  (atom)
  (quoted_atom)
  (keyword)
  (quoted_keyword)
] @string.special.symbol

; Note that we explicitly target sigil quoted start/end, so they are not overridden by delimiters

(sigil
  (sigil_name) @__name__
  quoted_start: _ @string.special
  quoted_end: _ @string.special) @string.special

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

; Calls

; * local function call
(call
  target: (identifier) @function)

; * remote function call
(call
  target: (dot
    right: (identifier) @function))

; * field without parentheses or block
(call
  target: (dot
    right: (identifier) @variable.other.member)
  .)

; * remote call without parentheses or block (overrides above)
(call
  target: (dot
    left: [
      (alias)
      (atom)
    ]
    right: (identifier) @function)
  .)

; * definition keyword
(call
  target: (identifier) @keyword
  (#any-of? @keyword "def" "defdelegate" "defexception" "defguard" "defguardp" "defimpl" "defmacro" "defmacrop" "defmodule" "defn" "defnp" "defoverridable" "defp" "defprotocol" "defstruct"))

; * kernel or special forms keyword
(call
  target: (identifier) @keyword
  (#any-of? @keyword "alias" "case" "cond" "for" "if" "import" "quote" "raise" "receive" "require" "reraise" "super" "throw" "try" "unless" "unquote" "unquote_splicing" "use" "with"))

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
  (#any-of? @keyword "def" "defdelegate" "defguard" "defguardp" "defmacro" "defmacrop" "defn" "defnp" "defp"))

; * pipe into identifier (function call)
(binary_operator
  operator: "|>"
  right: (identifier) @function)

; * pipe into identifier (definition)
(call
  target: (identifier) @keyword
  (arguments
    (binary_operator
      operator: "|>"
      right: (identifier) @variable))
  (#any-of? @keyword "def" "defdelegate" "defguard" "defguardp" "defmacro" "defmacrop" "defn" "defnp" "defp"))

; * pipe into field without parentheses (function call)
(binary_operator
  operator: "|>"
  right: (call
    target: (dot
      right: (identifier) @function)))

; Operators

; * capture operand
(unary_operator
  operator: "&"
  operand: [
    (integer) @operator
    (binary_operator
      left: [
        (call target: (dot left: (_) right: (identifier) @function))
        (identifier) @function
      ] operator: "/" right: (integer) @operator)
  ])

(operator_identifier) @operator

(unary_operator
  operator: _ @operator)

(binary_operator
  operator: _ @operator)

(dot
  operator: _ @operator)

(stab_clause
  operator: _ @operator)

; * module attribute
(unary_operator
  operator: "@" @variable.other.member
  operand: [
    (identifier) @variable.other.member
    (call
      target: (identifier) @variable.other.member)
    (boolean) @variable.other.member
    (nil) @variable.other.member
  ])

; * doc string
(unary_operator
  operator: "@" @comment.block.documentation
  operand: (call
    target: (identifier) @comment.block.documentation.__attribute__
    (arguments
      [
        (string) @comment.block.documentation
        (charlist) @comment.block.documentation
        (sigil
          quoted_start: _ @comment.block.documentation
          quoted_end: _ @comment.block.documentation) @comment.block.documentation
        (boolean) @comment.block.documentation
      ]))
  (#any-of? @comment.block.documentation.__attribute__ "moduledoc" "typedoc" "doc"))

; Module

(alias) @namespace

(call
  target: (dot
    left: (atom) @namespace))

; Reserved keywords

["when" "and" "or" "not" "in" "not in" "fn" "do" "end" "catch" "rescue" "after" "else"] @keyword
