[
  (footnote_marker_begin)
  (footnote_marker_end)
] @punctuation.bracket

(footnote_reference (reference_label) @markup.link.label)

(footnote (reference_label) @markup.link.label)

[
  (autolink)
  (inline_link_destination)
  (link_destination)
] @markup.link.url

(inline_link_destination
  [
    "("
    ")"
  ] @punctuation.bracket)

(link_reference_definition (link_label) @markup.link.label)

(link_reference_definition
  [
    "["
    "]"
  ] @punctuation.bracket)

(image_description) @label

(image_description
  [
    "!["
    "]"
  ] @punctuation.bracket)

(collapsed_reference_image "[]" @punctuation.bracket)

(full_reference_image
  [
    "["
    "]"
  ] @punctuation.bracket)

(full_reference_image (link_label) @markup.link.label)

(inline_link (link_text) @markup.link.text)

(collapsed_reference_link (link_text) @markup.link.text)

(full_reference_link
  [
    "["
    "]"
  ] @punctuation.bracket)

(collapsed_reference_link "[]" @punctuation.bracket)

(full_reference_link (link_label) @markup.link.label)

(full_reference_link (link_text) @markup.link.text)

(link_reference_definition ":" @punctuation.delimiter)

(inline_link (inline_link_destination) @markup.link.url)

(autolink
  [
    "<"
    ">"
  ] @punctuation.bracket)

(link_text
  [
    "["
    "]"
  ] @punctuation.bracket)

(key_value (value) @string)

(key_value (key) @attribute)

(key_value "=" @operator)

; NOTE: Not perfectly semantically accurate, but a fair approximation.
(identifier) @string.special.symbol

[
  (class)
  (class_name)
] @type

(block_attribute
  [
    "{"
    "}"
  ] @punctuation.bracket)

(inline_attribute
  [
    "{"
    "}"
  ] @punctuation.bracket)

(span
  [
    "["
    "]"
  ] @punctuation.bracket)

(inline_comment) @comment.line

(comment) @comment.block

(verbatim) @markup.raw

(raw_inline) @markup.raw

(math) @markup.raw

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

; TEMP: Scope not available, with no appropriate alternative.
(subscript) @markup.subscript

; TEMP: Scope not available, with no appropriate alternative.
(superscript) @markup.superscript

; TEMP: Scope not available, with no appropriate alternative.
(highlighted) @markup.highlighted

; TEMP: Scope not available, with no appropriate alternative.
(insert) @markup.insert

(delete) @markup.strikethrough

(symbol) @string.special.symbol

(strong) @markup.bold

(emphasis) @markup.italic

[
  (hard_line_break)
  (backslash_escape)
] @constant.character.escape

((quotation_marks) @constant.character.escape
  (#any-of? @constant.character.escape "\\\"" "\\'"))

(quotation_marks) @markup.quote

(list_item (term) @constructor)

[
  (ellipsis)
  (en_dash)
  (em_dash)
  (quotation_marks)
] @punctuation.special

(checked
  [
    "x"
    "X"
  ] @constant.builtin.boolean) @markup.list.checked

(list_marker_task
  (checked)) @markup.list.checked

(list_marker_task
  (unchecked)) @markup.list.unchecked

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

[
  (list_marker_dash)
  (list_marker_plus)
  (list_marker_star)
  (list_marker_definition)
] @markup.list.unnumbered

(table_caption) @label

(table_caption (marker) @punctuation.special)

(table_separator) @punctuation.special

(table_row "|" @punctuation.special)

(table_header "|" @punctuation.special)

(table_header) @markup.heading

[
  (block_quote)
  (block_quote_marker)
] @markup.quote

(language_marker) @punctuation.delimiter

(inline_attribute _ @attribute)

(language) @type.enum.variant

[
  (code_block_marker_begin)
  (code_block_marker_end)
  (raw_block_marker_begin)
  (raw_block_marker_end)
] @punctuation.bracket

[
  (code_block)
  (raw_block)
  (frontmatter)
] @markup.raw.block

[
  (div_marker_begin)
  (div_marker_end)
] @tag

(thematic_break) @special

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

(heading) @markup.heading
