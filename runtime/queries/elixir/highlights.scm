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

; Reserved keywords

["when" "and" "or" "not" "in" "not in" "fn" "do" "end" "catch" "rescue" "after" "else"] @keyword

; Operators

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
  (#match? @comment.block.documentation.__attribute__ "^(moduledoc|typedoc|doc)$"))

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

; * capture operator
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

; Literals

(nil) @constant.builtin

(boolean) @constant.builtin.boolean
(integer) @constant.numeric.integer
(float) @constant.numeric.float

(alias) @namespace

(call
  target: (dot
    left: (atom) @namespace))

(char) @constant.character

; Quoted content

(interpolation "#{" @punctuation.special "}" @punctuation.special) @embedded

(escape_sequence) @constant.character.escape

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
  quoted_start: _ @string.regexp
  quoted_end: _ @string.regexp
  (#match? @__name__ "^[rR]$")) @string.regexp

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
  (identifier) @comment
  (#match? @comment "^_")
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
