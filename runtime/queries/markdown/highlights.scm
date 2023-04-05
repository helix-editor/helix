
(setext_heading (paragraph) @markup.heading.1 (setext_h1_underline) @markup.heading.marker)
(setext_heading (paragraph) @markup.heading.2 (setext_h2_underline) @markup.heading.marker)

(atx_heading (atx_h1_marker) @markup.heading.marker (inline) @markup.heading.1)
(atx_heading (atx_h2_marker) @markup.heading.marker (inline) @markup.heading.2)
(atx_heading (atx_h3_marker) @markup.heading.marker (inline) @markup.heading.3)
(atx_heading (atx_h4_marker) @markup.heading.marker (inline) @markup.heading.4)
(atx_heading (atx_h5_marker) @markup.heading.marker (inline) @markup.heading.5)
(atx_heading (atx_h6_marker) @markup.heading.marker (inline) @markup.heading.6)

[
  (indented_code_block)
  (fenced_code_block)
] @markup.raw.block

(info_string) @label

[
  (fenced_code_block_delimiter)
] @punctuation.bracket

[
  (link_destination)
] @markup.link.url

[
  (link_label)
] @markup.link.label

[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
] @markup.list.unnumbered

[
  (list_marker_dot)
  (list_marker_parenthesis)
] @markup.list.numbered

(task_list_marker_checked) @markup.list.checked
(task_list_marker_unchecked) @markup.list.unchecked

(thematic_break) @punctuation.special

[
  (block_continuation)
  (block_quote_marker)
] @punctuation.special

[
  (backslash_escape)
] @string.escape

(block_quote) @markup.quote

(pipe_table_row
  "|" @punctuation.special)
(pipe_table_header
  "|" @punctuation.special)
(pipe_table_delimiter_row) @punctuation.special
