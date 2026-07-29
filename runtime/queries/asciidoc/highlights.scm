(document_title
  (title_h0_marker) @markup.heading.marker) @markup.heading.1

(title1
  (title_h1_marker) @markup.heading.marker) @markup.heading.2

(title2
  (title_h2_marker) @markup.heading.marker) @markup.heading.3

(title3
  (title_h3_marker) @markup.heading.marker) @markup.heading.4

(title4
  (title_h4_marker) @markup.heading.marker) @markup.heading.5

(title5
  (title_h5_marker) @markup.heading.marker) @markup.heading.6

(email) @markup.link @markup.link.url

(author_line
  ";" @punctuation.delimiter)

(revision_line
  "," @punctuation.delimiter
  ":" @punctuation.delimiter)

(list_continuation) @punctuation.special

[
  (firstname)
  (middlename)
  (lastname)
] @attribute

(revnumber) @constant.numeric

(revdate) @string.special

(revremark) @string

[
  (table_block_marker)
  (csv_table_block_marker)
  (dsv_table_block_marker)
] @punctuation.special

(table_cell_attr) @attribute

(table_cell
  "|" @punctuation.special)

(csv_record
  "," @punctuation.special)

(dsv_record
  ":" @punctuation.special)

[
  (breaks)
  (hard_wrap)
  (quoted_block_md_marker)
  (quoted_paragraph_marker)
  (open_block_marker)
  (listing_block_start_marker)
  (listing_block_end_marker)
  (literal_block_marker)
  (passthrough_block_marker)
  (quoted_block_start_marker)
  (quoted_block_end_marker)
  (sidebar_block_start_marker)
  (sidebar_block_end_marker)
  (ntable_block_marker)
  (callout_marker)
] @punctuation.special

(ntable_cell
  "!" @punctuation.special)

(checked_list_marker_unchecked) @markup.list.unchecked

(checked_list_marker_checked) @markup.list.checked

[
  (list_marker_star)
  (list_marker_hyphen)
] @markup.list.unnumbered

[
  (list_marker_dot)
  (list_marker_digit)
  (list_marker_geek)
  (list_marker_alpha)
] @markup.list.numbered

(description_marker) @markup.list.unnumbered

(description_list_item
  (term) @markup.bold)

[
  (line_comment)
  (block_comment)
] @comment @spell

[
  (document_attr_marker)
  (element_attr_marker)
] @punctuation.delimiter

(document_attr
  (attr_name) @attribute)

(block_style) @keyword
(positional_attr) @attribute
(id) @label
(role) @attribute
(option) @attribute

(block_title
  (block_title_marker) @punctuation.special) @markup.heading

(ident_block) @markup.raw.block

(callout_list_marker) @punctuation.special

(block_macro
  (block_macro_name) @function.macro
  "::" @punctuation.delimiter
  (target)? @markup.link.url
  "[" @punctuation.bracket
  "]" @punctuation.bracket)

(attribute_name) @attribute

(attribute_value) @string

(admonition
  (admonition_important) @keyword
  ":" @punctuation.delimiter)

(admonition
  (admonition_warning) @keyword
  ":" @punctuation.delimiter)

(admonition
  (admonition_caution) @keyword
  ":" @punctuation.delimiter)

(admonition
  (admonition_note) @keyword
  ":" @punctuation.delimiter)

(admonition
  (admonition_tip) @keyword
  ":" @punctuation.delimiter)

((section_block
  (element_attr
    (element_attr_marker) @punctuation.delimiter
    (positional_attr (block_style) @_style @keyword)
    (element_attr_marker) @punctuation.delimiter)
  (delimited_block
    (delimited_block_start_marker) @punctuation.special
    (delimited_block_end_marker) @punctuation.special))
  (#any-of? @_style "NOTE" "TIP"))

((section_block
  (element_attr
    (element_attr_marker) @punctuation.delimiter
    (positional_attr (block_style) @_style @keyword)
    (element_attr_marker) @punctuation.delimiter)
  (delimited_block
    (delimited_block_start_marker) @punctuation.special
    (delimited_block_end_marker) @punctuation.special))
  (#any-of? @_style "CAUTION" "WARNING"))

((section_block
  (element_attr
    (element_attr_marker) @punctuation.delimiter
    (positional_attr (block_style) @_style @keyword)
    (element_attr_marker) @punctuation.delimiter)
  (delimited_block
    (delimited_block_start_marker) @punctuation.special
    (delimited_block_end_marker) @punctuation.special))
  (#eq? @_style "IMPORTANT"))

((section_block
  (element_attr
    (positional_attr
      (block_style) @_style))
  (listing_block
    (listing_block_body) @markup.raw.block))
  (#not-any-of? @_style
    "a2s" "barcode" "blockdiag" "bpmn" "bytefield" "d2" "dbml" "diagrams" "ditaa" "dpic" "erd"
    "gnuplot" "graphviz" "lilypond" "meme" "mermaid" "msc" "nomnoml" "pikchr" "plantuml" "shaape"
    "smcat" "structurizr" "svgbob" "symbolator" "syntrax" "tikz" "umlet" "vega" "wavedrom"))

(paragraph) @spell
