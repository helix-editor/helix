["when" "and" "or" "not in" "not" "in" "fn" "do" "end" "catch" "rescue" "after" "else"] @keyword

[(true) (false) (nil)] @constant.builtin

(keyword
 [(keyword_literal)
  ":"] @tag)

(keyword
 (keyword_string
  [(string_start)
   (string_content)
   (string_end)] @tag))

[(atom_literal)
 (atom_start)
 (atom_content)
 (atom_end)] @tag

[(comment)
 (unused_identifier)] @comment

(escape_sequence) @escape

(call function: (function_identifier) @keyword
      (#match? @keyword "^(defmodule|defexception|defp|def|with|case|cond|raise|import|require|use|defmacrop|defmacro|defguardp|defguard|defdelegate|defstruct|alias|defimpl|defprotocol|defoverridable|receive|if|for|try|throw|unless|reraise|super|quote|unquote|unquote_splicing)$"))

(call function: (function_identifier) @keyword
      [(call
        function: (function_identifier) @function
        (arguments
         [(identifier) @variable.parameter
          (_ (identifier) @variable.parameter)
          (_ (_ (identifier) @variable.parameter))
          (_ (_ (_ (identifier) @variable.parameter)))
          (_ (_ (_ (_ (identifier) @variable.parameter))))
          (_ (_ (_ (_ (_ (identifier) @variable.parameter)))))]))
       (binary_op
        left:
        (call
         function: (function_identifier) @function
         (arguments
          [(identifier) @variable.parameter
           (_ (identifier) @variable.parameter)
           (_ (_ (identifier) @variable.parameter))
           (_ (_ (_ (identifier) @variable.parameter)))
           (_ (_ (_ (_ (identifier) @variable.parameter))))
           (_ (_ (_ (_ (_ (identifier) @variable.parameter)))))]))
        operator: "when")
       (binary_op
        left: (identifier) @variable.parameter
        operator: _ @function
        right: (identifier) @variable.parameter)]
      (#match? @keyword "^(defp|def|defmacrop|defmacro|defguardp|defguard|defdelegate)$"))

(call (function_identifier) @keyword
      [(call
        function: (function_identifier) @function)
       (identifier) @function
       (binary_op
        left:
        [(call
          function: (function_identifier) @function)
         (identifier) @function]
        operator: "when")]
      (#match? @keyword "^(defp|def|defmacrop|defmacro|defguardp|defguard|defdelegate)$"))

(anonymous_function
 (stab_expression
  left: (bare_arguments
         [(identifier) @variable.parameter
          (_ (identifier) @variable.parameter)
          (_ (_ (identifier) @variable.parameter))
          (_ (_ (_ (identifier) @variable.parameter)))
          (_ (_ (_ (_ (identifier) @variable.parameter))))
          (_ (_ (_ (_ (_ (identifier) @variable.parameter)))))])))

(unary_op
 operator: "@"
 (call (identifier) @attribute
       (heredoc
        [(heredoc_start)
         (heredoc_content)
         (heredoc_end)] @doc))
 (#match? @attribute "^(doc|moduledoc)$"))

(module) @type

(unary_op
 operator: "@" @attribute
 [(call
   function: (function_identifier) @attribute)
  (identifier) @attribute])

(unary_op
 operator: _ @operator)

(binary_op
 operator: _ @operator)

(heredoc
 [(heredoc_start)
  (heredoc_content)
  (heredoc_end)] @string)

(string
 [(string_start)
  (string_content)
  (string_end)] @string)

(sigil_start) @string.special
(sigil_content) @string
(sigil_end) @string.special

(interpolation
 "#{" @punctuation.special
 "}" @punctuation.special)

[
 ","
 "->"
 "."
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

(special_identifier) @function.special

(ERROR) @warning
