(function ((identifier) @function))
; method calls
(term (_) ("." @punctuation) ((function ((identifier) @function.method))))

["(" ")"] @punctuation.bracket
"," @punctuation.delimiter

((identifier) @keyword.control.conditional (#eq? @keyword.control.conditional "if"))
((identifier) @keyword.control.repeat (#eq? @keyword.control.repeat "for"))

(term ((identifier) @variable))

[(infix_ops) "++"] @operator
[(string_literal) (raw_string_literal)] @string

(integer_literal) @constant.numeric.integer
