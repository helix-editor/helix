[
  (comment)
  (extra_text)
] @comment

[
  (section_header)
  (setting_statement)
  (keyword_setting)
  (test_case_setting)
] @keyword

(variable_definition (variable_name) @variable)
(keyword_definition (name) @function)
(test_case_definition (name) @function)

(keyword_invocation (keyword) @function)
(ellipses) @punctuation.delimiter

(text_chunk) @string
(inline_python_expression) @string.special
[
  (scalar_variable)
  (list_variable)
  (dictionary_variable)
] @variable

; Control structures

"RETURN" @keyword.control.return

[
  "FOR"
  "IN"
  "IN RANGE"
  "IN ENUMERATE"
  "IN ZIP"
  (break_statement)
  (continue_statement)
] @keyword.control.repeat
(for_statement "END" @keyword.control.repeat)

"WHILE" @keyword.control.repeat
(while_statement "END" @keyword.control.repeat)

[
  "IF"
  "ELSE IF"
] @keyword.control.conditional
(if_statement "END" @keyword.control.conditional)
(if_statement (else_statement "ELSE" @keyword.control.conditional))

[
  "TRY"
  "EXCEPT"
  "FINALLY"
] @keyword.control.exception
(try_statement "END" @keyword.control.exception)
(try_statement (else_statement "ELSE" @keyword.control.exception))
