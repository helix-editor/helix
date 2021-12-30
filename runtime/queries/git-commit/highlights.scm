(subject) @markup.heading
(path) @string.special.path
(branch) @string.special.symbol
(commit) @constant
(item) @markup.link.url
(header) @tag

(change kind: "new file" @diff.plus)
(change kind: "deleted" @diff.minus)
(change kind: "modified" @diff.delta)
(change kind: "renamed" @diff.delta.moved)

[":" "->"] @punctuation.delimeter
(comment) @comment
