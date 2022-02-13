(operation operator: ["p" "pick" "r" "reword" "e" "edit" "s" "squash" "m" "merge" "d" "drop" "b" "break" "x" "exec"] @keyword)
(operation operator: ["l" "label" "t" "reset"] @function)
(operation operator: ["f" "fixup"] @function.special)

(option) @operator
(label) @string.special.symbol
(commit) @constant
"#" @punctuation.delimiter
(comment) @comment

(ERROR) @error
