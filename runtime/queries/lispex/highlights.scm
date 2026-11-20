(line_comment) @comment
(block_comment) @comment
(forbidden_reader_extension) @diagnostic.error

(boolean) @constant.builtin.boolean
(integer) @constant.numeric.integer
(rational) @constant.numeric.integer
(real) @constant.numeric.float
(character) @constant.character
(string) @string
(byte) @constant.numeric.integer

[
  "'"
  "`"
  ","
  ",@"
  "."
] @operator

((symbol) @keyword
  (#match? @keyword "^(lambda|if|begin|set!|define|let|let\\*|letrec|quote|quasiquote|unquote|unquote-splicing|values|call-with-values|call/cc|dynamic-wind|cond|case|and|or|when|unless|do|guard|module|export|import)$"))

((list
  . [(line_comment) (block_comment)]* .
  (datum
    (symbol) @function))
  (#not-match? @function "^(lambda|if|begin|set!|define|let|let\\*|letrec|quote|quasiquote|unquote|unquote-splicing|values|call-with-values|call/cc|dynamic-wind|cond|case|and|or|when|unless|do|guard|module|export|import)$"))

((dotted_list
  . [(line_comment) (block_comment)]* .
  (datum
    (symbol) @function))
  (#not-match? @function "^(lambda|if|begin|set!|define|let|let\\*|letrec|quote|quasiquote|unquote|unquote-splicing|values|call-with-values|call/cc|dynamic-wind|cond|case|and|or|when|unless|do|guard|module|export|import)$"))


["(" ")" "#(" "#u8("] @punctuation.bracket
