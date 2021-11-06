
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
  (identifier) @variable.other.member .)

(function_definition
  name: (identifier) @function)
(call_expression
  (identifier) @function)
(call_expression
  (field_expression (identifier) @function.method .))
(broadcast_call_expression
  (identifier) @function)
(broadcast_call_expression
  (field_expression (identifier) @function.method .))
(parameter_list
  (identifier) @variable.parameter)
(parameter_list
  (optional_parameter .
    (identifier) @variable.parameter))
(typed_parameter
  (identifier) @variable.parameter
  (identifier) @type)
(type_parameter_list
  (identifier) @type)
(typed_parameter
  (identifier) @variable.parameter
  (parameterized_identifier) @type)
(function_expression
  . (identifier) @variable.parameter)
(spread_parameter) @variable.parameter
(spread_parameter
  (identifier) @variable.parameter)
(named_argument
    . (identifier) @variable.parameter)
(argument_list
  (typed_expression
    (identifier) @variable.parameter
    (identifier) @type))
(argument_list
  (typed_expression
    (identifier) @variable.parameter
    (parameterized_identifier) @type))

;; Symbol expressions (:my-wanna-be-lisp-keyword)
(quote_expression
 (identifier)) @string.special.symbol

;; Parsing error! foo (::Type) get's parsed as two quote expressions
(argument_list 
  (quote_expression
    (quote_expression
      (identifier) @type)))

(type_argument_list
  (identifier) @type)
(parameterized_identifier (_)) @type
(argument_list
  (typed_expression . (identifier) @variable.parameter))

(typed_expression
  (identifier) @type .)
(typed_expression
  (parameterized_identifier) @type .)

(struct_definition
  name: (identifier) @type)

(number) @constant.numeric.integer
(range_expression
    (identifier) @constant.numeric.integer
      (eq? @constant.numeric.integer "end"))
(range_expression
  (_
    (identifier) @constant.numeric.integer
      (eq? @constant.numeric.integer "end")))
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
  ["if" "end"] @keyword.control.conditional)
(elseif_clause
  ["elseif"] @keyword.control.conditional)
(else_clause
  ["else"] @keyword.control.conditional)
(ternary_expression
  ["?" ":"] @keyword.control.conditional)

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

((identifier) @keyword (match? @keyword "global|local"))

(compound_expression
  ["begin" "end"] @keyword)
(try_statement
  ["try" "end" ] @keyword.control.exception)
(finally_clause
  "finally" @keyword.control.exception)
(catch_clause
  "catch" @keyword.control.exception)
(quote_statement
  ["quote" "end"] @keyword)
(let_statement
  ["let" "end"] @keyword)
(for_statement
  ["for" "end"] @keyword.control.repeat)
(while_statement
  ["while" "end"] @keyword.control.repeat)
(break_statement) @keyword.control.repeat
(continue_statement) @keyword.control.repeat
(for_binding
  "in" @keyword.control.repeat)
(for_clause
  "for" @keyword.control.repeat)
(do_clause
  ["do" "end"] @keyword)

(export_statement
  ["export"] @keyword.control.import)

[
  "using"
  "module"
  "import"
] @keyword.control.import

((identifier) @keyword.control.import (#eq? @keyword.control.import "baremodule"))

(((identifier) @constant.builtin) (match? @constant.builtin "^(nothing|Inf|NaN)$"))
(((identifier) @constant.builtin.boolean) (#eq? @constant.builtin.boolean "true"))
(((identifier) @constant.builtin.boolean) (#eq? @constant.builtin.boolean "false"))


["::" ":" "." "," "..." "!"] @punctuation.delimiter
["[" "]" "(" ")" "{" "}"] @punctuation.bracket

["="] @operator

(identifier) @variable
;; In case you want type highlighting based on Julia naming conventions (this might collide with mathematical notation)
;((identifier) @type ; exception: mark `A_foo` sort of identifiers as variables
  ;(match? @type "^[A-Z][^_]"))
((identifier) @constant
  (match? @constant "^[A-Z][A-Z_]{2}[A-Z_]*$"))
