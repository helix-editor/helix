; Variables
(variable_declaration
  .
  (scope) @keyword)
[
(single_var_declaration)
(scalar_variable)
(array_variable)
(hash_variable)
(hash_variable)
] @variable


[
(package_name)
(special_scalar_variable)
(special_array_variable)
(special_hash_variable)
(special_literal)
(super)
] @constant

(
  [
  (package_name)
  (super)
  ]
  .
  ("::" @operator)
)

(comments) @comment
(pod_statement) @comment.block.documentation

[
(use_no_statement)
(use_no_feature_statement)
(use_no_if_statement)
(use_no_version)
(use_constant_statement)
(use_parent_statement)
] @keyword

(use_constant_statement
  constant: (identifier) @constant)

[
"require"
] @keyword

(method_invocation
  .
  (identifier) @variable)

(method_invocation
  (arrow_operator)
  .
  (identifier) @function)
(method_invocation
  function_name: (identifier) @function)
(named_block_statement
  function_name: (identifier) @function)

(call_expression
  function_name: (identifier) @function)
(function_definition
  name: (identifier) @function)
[
(function)
(map)
(grep)
(bless)
] @function

[
"return"
"sub"
"package"
"BEGIN"
"END"
] @keyword.function

[
"("
")"
"["
"]"
"{"
"}"
] @punctuation.bracket
(standard_input_to_variable) @punctuation.bracket

[
"=~"
"or"
"="
"=="
"+"
"-"
"."
"//"
"||"
(arrow_operator)
(hash_arrow_operator)
(array_dereference)
(hash_dereference)
(to_reference)
(type_glob)
(hash_access_variable)
(ternary_expression)
(ternary_expression_in_hash)
] @operator

[
(regex_option)
(regex_option_for_substitution)
(regex_option_for_transliteration)
] @variable.parameter

(type_glob
  (identifier) @variable)
(
  (scalar_variable)
  .
  ("->" @operator))

[
(word_list_qw)
(command_qx_quoted)
(string_single_quoted)
(string_double_quoted)
(string_qq_quoted)
(bareword)
(transliteration_tr_or_y)
] @string

[
(regex_pattern_qr) 
(patter_matcher_m)
(substitution_pattern_s)
] @string.regexp

(escape_sequence) @string.special

[
","
(semi_colon)
(start_delimiter)
(end_delimiter)
(ellipsis_statement)
] @punctuation.delimiter

[
(integer)
(floating_point)
(scientific_notation)
(hexadecimal)
] @constant.numeric

[
; (if_statement)
(unless_statement)
(if_simple_statement)
(unless_simple_statement)
] @keyword.control.conditional

[
"if"
"elsif"
"else"
] @keyword.control.conditional 

(foreach_statement) @keyword.control.repeat
(foreach_statement
  .
  (scope) @keyword)

(function_attribute) @label

(function_signature) @type

