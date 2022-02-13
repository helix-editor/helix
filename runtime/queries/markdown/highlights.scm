[
  (atx_heading)
  (setext_heading)
] @markup.heading

(code_fence_content) @none

[
  (indented_code_block)
  (fenced_code_block)
] @markup.raw.block

(block_quote) @markup.quote

(code_span) @markup.raw.inline

(emphasis) @markup.italic

(strong_emphasis) @markup.bold

(link_destination) @markup.link.url
(link_label) @markup.link.label

[
  (link_text)
  (image_description)
] @markup.link.text

[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
  (list_marker_dot)
  (list_marker_parenthesis)
] @punctuation.special

[
  (backslash_escape)
  (hard_line_break)
] @string.character.escape

