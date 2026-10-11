((block_macro
  (block_macro_name)
  (target) @injection.content)
  (#set! injection.language "asciidoc_inline"))

((table_cell
  (table_cell_content) @injection.content)
  (#set! injection.language "asciidoc_inline"))

((paragraph) @injection.content
  (#set! injection.include-children)
  (#set! injection.language "asciidoc_inline"))

((line) @injection.content
  (#set! injection.include-children)
  (#set! injection.language "asciidoc_inline"))

; Source listing: `[source,ruby]` - the language is the second positional.
((section_block
  (element_attr
    (positional_attr
      (block_style))
    (positional_attr) @injection.language)
  (listing_block
    (listing_block_start_marker)
    (listing_block_body) @injection.content
    (listing_block_end_marker))))

; Diagram listing: `[mermaid]` - the language is the style itself.
((section_block
  (element_attr
    (positional_attr
      (block_style) @injection.language))
  (listing_block
    (listing_block_start_marker)
    (listing_block_body) @injection.content
    (listing_block_end_marker)))
  (#any-of? @injection.language
    "a2s" "barcode" "blockdiag" "bpmn" "bytefield" "d2" "dbml" "diagrams" "ditaa" "dpic" "erd"
    "gnuplot" "graphviz" "lilypond" "meme" "mermaid" "msc" "nomnoml" "pikchr" "plantuml" "shaape"
    "smcat" "structurizr" "svgbob" "symbolator" "syntrax" "tikz" "umlet" "vega" "wavedrom"))

; Source paragraph: `[source,ruby]` over a paragraph.
((section_block
  (element_attr
    (positional_attr
      (block_style))
    (positional_attr) @injection.language)
  (paragraph) @injection.content)
  (#set! injection.include-children))

; latexmath passthrough block, mapped onto the latex grammar.
(section_block
  (element_attr
    (positional_attr
      (block_style) @_style))
  (passthrough_block
    (passthrough_block_marker)
    (listing_block_body) @injection.content
    (passthrough_block_marker))
  (#any-of? @_style "latexmath")
  (#set! injection.language "latex"))
