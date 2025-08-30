; inherits: html

[
  (if_statement)
  (each_statement)
  (await_statement)
  (key_statement)
  (snippet_statement)
] @indent.begin

(if_end
  "}" @indent.end)

(each_end
  "}" @indent.end)

(await_end
  "}" @indent.end)

(key_end
  "}" @indent.end)

(snippet_end
  "}" @indent.end)

[
  (if_end)
  (else_if_block)
  (else_block)
  (each_end)
  (await_end)
  (key_end)
  (snippet_end)
] @indent.branch
