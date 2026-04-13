; inherits: html

[
 (statement_block)
 (switch_statement)
] @indent.begin

(statement_block
 "{" @indent.branch)

(statement_block
 "}" @indent.end)

"}" @indent.branch

"}" @indent.end
