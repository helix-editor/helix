(setext_heading (heading_content) @markup.heading.1 (setext_h1_underline) @markup.heading.marker)
(setext_heading (heading_content) @markup.heading.2 (setext_h2_underline) @markup.heading.marker)

(atx_heading (atx_h1_marker) @markup.heading.marker (heading_content) @markup.heading.1)
(atx_heading (atx_h2_marker) @markup.heading.marker (heading_content) @markup.heading.2)
(atx_heading (atx_h3_marker) @markup.heading.marker (heading_content) @markup.heading.3)
(atx_heading (atx_h4_marker) @markup.heading.marker (heading_content) @markup.heading.4)
(atx_heading (atx_h5_marker) @markup.heading.marker (heading_content) @markup.heading.5)
(atx_heading (atx_h6_marker) @markup.heading.marker (heading_content) @markup.heading.6)

[
  (indented_code_block)
  (code_fence_content)
] @markup.raw.block

(block_quote) @markup.quote

(code_span) @markup.raw.inline

(emphasis) @markup.italic

(strong_emphasis) @markup.bold

(link_destination) @markup.link.url
(link_label) @markup.link.label

(info_string) @label

[
  (link_text)
  (image_description)
] @markup.link.text

[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
] @markup.list.numbered

[
  (list_marker_dot)
  (list_marker_parenthesis)
] @markup.list.unnumbered

[
  (backslash_escape)
  (hard_line_break)
] @constant.character.escape

(thematic_break) @punctuation.delimiter

(inline_link ["[" "]" "(" ")"] @punctuation.bracket)
(image ["[" "]" "(" ")"] @punctuation.bracket)
(fenced_code_block_delimiter) @punctuation.bracket
