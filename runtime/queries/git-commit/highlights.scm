[(comment) (generated_comment) (scissor)] @comment
(subject) @markup.heading
(branch) @string.special.symbol
(filepath) @string.special.path
(arrow) @punctuation.delimiter
(subject (subject_prefix) @function)
(prefix (type) @keyword)
(prefix (scope) @variable.parameter)
(prefix [ "(" ")" ":" ] @punctuation.delimiter)
(prefix "!" @punctuation.special)
(trailer (token) @variable.other.member)
(trailer (value) @string)
(breaking_change (token) @special)

(change kind: (new)) @diff.plus
(change kind: (deleted)) @diff.minus
(change kind: (modified)) @diff.delta
(change kind: [(renamed) (typechange)]) @diff.delta.moved
