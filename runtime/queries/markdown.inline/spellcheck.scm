; Markdown prose is injected into the `markdown.inline` layer as a single
; `(inline)` node, so checking that covers paragraphs, headings, list items and
; emphasised/linked text alike. The non-prose spans are then carved back out:
; inline code and link/autolink URLs. Visible link text is left in.
(inline) @spell

[
  (code_span)
  (link_destination)
  (uri_autolink)
] @nospell
