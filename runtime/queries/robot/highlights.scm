(comment) @comment
(ellipses) @punctuation.delimiter

(section_header) @keyword
(extra_text) @comment

(setting_statement) @keyword

(variable_definition (variable_name) @variable)

(keyword_definition (name) @function)
(keyword_definition (body (keyword_setting) @keyword))

(test_case_definition (name) @property)

(keyword_invocation (keyword) @function)

(argument (text_chunk) @string)
(argument (scalar_variable) @string.special)
(argument (list_variable) @string.special)
(argument (dictionary_variable) @string.special)
