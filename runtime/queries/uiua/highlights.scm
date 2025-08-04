(comment) @comment
(string) @string
(number) @constant.numeric

(identifier) @variable

(builtin_func0) @constant
(builtin_func1) @function.builtin
(builtin_func2) @operator
; (builtin_func3)

(builtin_macro1) @function.macro
(builtin_macro2) @keyword
; (builtin_macro3)

(func (_)) @function

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["|" "_"] @punctuation.delimiter
