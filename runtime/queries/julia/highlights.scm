(identifier) @variable
;; In case you want type highlighting based on Julia naming conventions (this might collide with mathematical notation)
;((identifier) @type ; exception: mark `A_foo` sort of identifiers as variables
  ;(match? @type "^[A-Z][^_]"))
((identifier) @constant
  (match? @constant "^[A-Z][A-Z_]{2}[A-Z_]*$"))

[
  (triple_string)
  (string)
] @string

(string
  prefix: (identifier) @constant.builtin)

(macro_identifier) @function.macro
(macro_identifier (identifier) @function.macro) ; for any one using the variable highlight
(macro_definition
  name: (identifier) @function.macro
  ["macro" "end" @keyword])

(field_expression
  (identifier)
  (identifier) @field .)

(function_definition
  name: (identifier) @function)
(call_expression
  (identifier) @function)
(call_expression
  (field_expression (identifier) @method .))
(broadcast_call_expression
  (identifier) @function)
(broadcast_call_expression
  (field_expression (identifier) @method .))
(parameter_list
  (identifier) @parameter)
(parameter_list
  (optional_parameter .
    (identifier) @parameter))
(typed_parameter
  (identifier) @parameter
  (identifier) @type)
(type_parameter_list
  (identifier) @type)
(typed_parameter
  (identifier) @parameter
  (parameterized_identifier) @type)
(function_expression
  . (identifier) @parameter)
(spread_parameter) @parameter
(spread_parameter
  (identifier) @parameter)
(named_argument
    . (identifier) @parameter)
(argument_list
  (typed_expression
    (identifier) @parameter
    (identifier) @type))
(argument_list
  (typed_expression
    (identifier) @parameter
    (parameterized_identifier) @type))

;; Symbol expressions (:my-wanna-be-lisp-keyword)
(quote_expression
 (identifier)) @symbol

;; Parsing error! foo (::Type) get's parsed as two quote expressions
(argument_list 
  (quote_expression
    (quote_expression
      (identifier) @type)))

(type_argument_list
  (identifier) @type)
(parameterized_identifier (_)) @type
(argument_list
  (typed_expression . (identifier) @parameter))

(typed_expression
  (identifier) @type .)
(typed_expression
  (parameterized_identifier) @type .)

(struct_definition
  name: (identifier) @type)

(number) @number
(range_expression
    (identifier) @number
      (eq? @number "end"))
(range_expression
  (_
    (identifier) @number
      (eq? @number "end")))
(coefficient_expression
  (number)
  (identifier) @constant.builtin)

;; TODO: operators.
;; Those are a bit difficult to implement since the respective nodes are hidden right now (_power_operator)
;; and heavily use Unicode chars (support for those are bad in vim/lua regexes)
;[;
    ;(power_operator);
    ;(times_operator);
    ;(plus_operator);
    ;(arrow_operator);
    ;(comparison_operator);
    ;(assign_operator);
;] @operator ;

"end" @keyword

(if_statement
  ["if" "end"] @conditional)
(elseif_clause
  ["elseif"] @conditional)
(else_clause
  ["else"] @conditional)
(ternary_expression
  ["?" ":"] @conditional)

(function_definition ["function" "end"] @keyword.function)

(comment) @comment

[
  "const"
  "return"
  "macro"
  "struct"
  "primitive"
  "type"
] @keyword

((identifier) @keyword (#any-of? @keyword "global" "local"))

(compound_expression
  ["begin" "end"] @keyword)
(try_statement
  ["try" "end" ] @exception)
(finally_clause
  "finally" @exception)
(catch_clause
  "catch" @exception)
(quote_statement
  ["quote" "end"] @keyword)
(let_statement
  ["let" "end"] @keyword)
(for_statement
  ["for" "end"] @repeat)
(while_statement
  ["while" "end"] @repeat)
(break_statement) @repeat
(continue_statement) @repeat
(for_binding
  "in" @repeat)
(for_clause
  "for" @repeat)
(do_clause
  ["do" "end"] @keyword)

(export_statement
  ["export"] @include)

[
  "using"
  "module"
  "import"
] @include

((identifier) @include (#eq? @include "baremodule"))

(((identifier) @constant.builtin) (match? @constant.builtin "^(nothing|Inf|NaN)$"))
(((identifier) @boolean) (eq? @boolean "true"))
(((identifier) @boolean) (eq? @boolean "false"))

["::" ":" "." "," "..." "!"] @punctuation.delimiter
["[" "]" "(" ")" "{" "}"] @punctuation.bracket
