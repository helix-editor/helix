(heading) @markup.heading

((heading
  (marker) @markup.heading.marker) @markup.heading.1
  (#eq? @markup.heading.marker "# "))

((heading
  (marker) @markup.heading.marker) @markup.heading.2
  (#eq? @markup.heading.marker "## "))

((heading
  (marker) @markup.heading.marker) @markup.heading.3
  (#eq? @markup.heading.marker "### "))

((heading
  (marker) @markup.heading.marker) @markup.heading.4
  (#eq? @markup.heading.marker "##### "))

((heading
  (marker) @markup.heading.marker) @markup.heading.5
  (#eq? @markup.heading.marker "###### "))

((heading
  (marker) @markup.heading.marker) @markup.heading.6
  (#eq? @markup.heading.marker "####### "))

(thematic_break) @special

[
  (div_marker_begin)
  (div_marker_end)
] @tag

[
  (code_block)
  (raw_block)
  (frontmatter)
] @markup.raw.block

[
  (code_block_marker_begin)
  (code_block_marker_end)
  (raw_block_marker_begin)
  (raw_block_marker_end)
] @punctuation.bracket

(language) @type.enum.variant

(inline_attribute _ @attribute)

(language_marker) @punctuation.delimiter

[
  (block_quote)
  (block_quote_marker)
] @markup.quote

(table_header) @markup.heading

(table_header "|" @punctuation.special)

(table_row "|" @punctuation.special)

(table_separator) @punctuation.special

(table_caption (marker) @punctuation.special)

(table_caption) @label

[
  (list_marker_dash)
  (list_marker_plus)
  (list_marker_star)
  (list_marker_definition)
] @markup.list.unnumbered

[
  (list_marker_decimal_period)
  (list_marker_decimal_paren)
  (list_marker_decimal_parens)
  (list_marker_lower_alpha_period)
  (list_marker_lower_alpha_paren)
  (list_marker_lower_alpha_parens)
  (list_marker_upper_alpha_period)
  (list_marker_upper_alpha_paren)
  (list_marker_upper_alpha_parens)
  (list_marker_lower_roman_period)
  (list_marker_lower_roman_paren)
  (list_marker_lower_roman_parens)
  (list_marker_upper_roman_period)
  (list_marker_upper_roman_paren)
  (list_marker_upper_roman_parens)
] @markup.list.numbered

(list_marker_task
  (unchecked)) @markup.list.unchecked

(list_marker_task
  (checked)) @markup.list.checked

(checked
  [
    "x"
    "X"
  ] @constant.builtin.boolean) @markup.list.checked

[
  (ellipsis)
  (en_dash)
  (em_dash)
  (quotation_marks)
] @punctuation.special

(list_item (term) @constructor)

(quotation_marks) @markup.quote

((quotation_marks) @constant.character.escape
  (#any-of? @constant.character.escape "\\\"" "\\'"))

[
  (hard_line_break)
  (backslash_escape)
] @constant.character.escape

(emphasis) @markup.italic

(strong) @markup.bold

(symbol) @string.special.symbol

(delete) @markup.strikethrough

(insert) @markup.italic

(highlighted) @markup.bold

(superscript) @string.special.superscript

(subscript) @string.special.subscript

[
  (emphasis_begin)
  (emphasis_end)
  (strong_begin)
  (strong_end)
  (superscript_begin)
  (superscript_end)
  (subscript_begin)
  (subscript_end)
  (highlighted_begin)
  (highlighted_end)
  (insert_begin)
  (insert_end)
  (delete_begin)
  (delete_end)
  (verbatim_marker_begin)
  (verbatim_marker_end)
  (math_marker)
  (math_marker_begin)
  (math_marker_end)
  (raw_inline_attribute)
  (raw_inline_marker_begin)
  (raw_inline_marker_end)
] @punctuation.bracket

(math) @markup.raw

(verbatim) @markup.raw

(raw_inline) @markup.raw

(comment) @comment.block

(inline_comment) @comment.line

(span
  [
    "["
    "]"
  ] @punctuation.bracket)

(inline_attribute
  [
    "{"
    "}"
  ] @punctuation.bracket)

(block_attribute
  [
    "{"
    "}"
  ] @punctuation.bracket)

[
  (class)
  (class_name)
] @type

; NOTE: Not perfectly semantically accurate, but a fair approximation.
(identifier) @string.special.symbol

(key_value "=" @operator)

(key_value (key) @attribute)

(key_value (value) @string)

(link_text
  [
    "["
    "]"
  ] @punctuation.bracket)

(autolink
  [
    "<"
    ">"
  ] @punctuation.bracket)

(inline_link (inline_link_destination) @markup.link.url)

(link_reference_definition ":" @punctuation.delimiter)

(full_reference_link (link_text) @markup.link.text)

(full_reference_link (link_label) @markup.link.label)

(collapsed_reference_link "[]" @punctuation.bracket)

(full_reference_link
  [
    "["
    "]"
  ] @punctuation.bracket)

(collapsed_reference_link (link_text) @markup.link.text)

(inline_link (link_text) @markup.link.text)

(full_reference_image (link_label) @markup.link.label)

(full_reference_image
  [
    "["
    "]"
  ] @punctuation.bracket)

(collapsed_reference_image "[]" @punctuation.bracket)

(image_description
  [
    "!["
    "]"
  ] @punctuation.bracket)

(image_description) @label

(link_reference_definition
  [
    "["
    "]"
  ] @punctuation.bracket)

(link_reference_definition (link_label) @markup.link.label)

(inline_link_destination
  [
    "("
    ")"
  ] @punctuation.bracket)

[
  (autolink)
  (inline_link_destination)
  (link_destination)
] @markup.link.url

(footnote (reference_label) @markup.link.label)

(footnote_reference (reference_label) @markup.link.label)

[
  (footnote_marker_begin)
  (footnote_marker_end)
] @punctuation.bracket
