; Preamble
(preamble
  (title) @markup.heading.1
  (section) @constant.numeric)

(footer_text) @string

; Headings
(heading) @markup.heading.1

(subheading) @markup.heading.2

; Formatting
(bold
  "*" @punctuation.special
  (bold_content) @markup.bold)

(underline
  "_" @punctuation.special
  (underline_content) @markup.italic)

(inline_code
  "`" @punctuation.special
  (code_content) @markup.raw.inline)

; Code blocks
(literal_block) @markup.raw.block

(code_block_content) @markup.raw.block

; Lists
(list_item) @markup.list.unnumbered

(numbered_list_item) @markup.list.numbered

; Tables
(table_row) @markup.raw

; Comments
(comment) @comment

; Line break
(line_break) @punctuation.special

; Escape sequence
(escape_sequence) @string.escape

; Punctuation
["(" ")" "\""] @punctuation.bracket
