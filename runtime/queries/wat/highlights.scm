[
  "module" "func" "param" "result" "type" "memory" "elem" "data" "table" "global"
  "if" "then" "else" "block" "loop" "end" "mut"
] @keyword

["import" "export"] @keyword.control.import

["local"] @keyword.storage.type

[(name) (string)] @string

(identifier) @function

[(comment_block) (comment_line)] @comment

[(nat) (float) (align_offset_value)] @constant.numeric.integer

(value_type) @type

["(" ")"] @punctuation.bracket
