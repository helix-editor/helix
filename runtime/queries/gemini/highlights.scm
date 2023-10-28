(link) @punctuation.bracket
(link 
  label: (text) @markup.link.label)
(link
  uri: (uri) @markup.link.url)

[
  (start_pre)
  (pre)
  (end_pre)
] @markup.raw.block
(start_pre
  alt: (text) @label)

(heading1
  (text) @markup.heading.1) @markup.heading.marker
(heading2
  (text) @markup.heading.2) @markup.heading.marker
(heading3
  (text) @markup.heading.3) @markup.heading.marker

(ulist
  (indicator) @markup.list.unnumbered) 
(quote
  (indicator) @markup.quote
  (text) @markup.italic)
