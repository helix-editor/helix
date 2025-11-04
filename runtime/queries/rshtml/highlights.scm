(start_symbol) @keyword
(hash_symbol) @punctuation.special

(open_paren) @punctuation.bracket
(close_paren) @punctuation.bracket
(open_brace) @punctuation.bracket
(close_brace) @punctuation.bracket

(fat_arrow) @operator
(semicolon) @punctuation.delimiter
(equals) @punctuation.delimiter

(string_line) @string

(comment_block) @comment.block
(open_comment) @operator
(close_comment) @operator

(continue_) @keyword.control.conditional
(break_) @keyword.control.conditional

(
  (start_symbol) @keyword.control.import
  .
  (extends_) @keyword.control.import
)

(raw_) @keyword

(
  (start_symbol) @keyword.control.import
  .
  (include_directive (include_) @keyword.control.import)
)

(render_) @keyword
(render_body_) @keyword
(child_content_) @keyword
(section_) @keyword

(section_block
  name: (rust_identifier) @namespace)

(as_) @keyword.operator
(as_clause
  alias: (rust_identifier) @type)
(
  (start_symbol) @keyword.control.import
  .
  (use_directive (use_) @keyword.control.import)
)

(number) @constant.numeric
(bool) @constant.builtin.boolean

(tag_open) @punctuation.bracket
(tag_close) @punctuation.bracket
(tag_end_open) @punctuation.bracket
(tag_self_close) @punctuation.bracket

(component_tag
  name: (component_tag_identifier) @tag)

(component_tag
  name_close: (component_tag_identifier) @tag)

(component_tag_parameter
  name: (rust_identifier) @attribute)

(
  (start_symbol) @function.method
  .
  (rust_expr_simple)
)

(
  (start_symbol) @function.method
  .
  (rust_expr_paren)
)

(
  (start_symbol) @keyword.directive
  .
  (rust_block)
)

(
  (start_symbol) @keyword.control.conditional
  .
  (if_stmt)
)
(
  (start_symbol) @keyword
  .
  (for_stmt)
)
(
  (start_symbol) @keyword.control.repeat
  .
  (while_stmt)
)
(
  (start_symbol) @keyword.control.conditional
  .
  (match_stmt)
)

;this is for now extra
(else_clause
  head: (rust_text) @keyword.control.conditional)
