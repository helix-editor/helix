[(pod_directive)
 (head_directive)
 (over_directive)
 (item_directive)
 (back_directive)
 (encoding_directive)
 (cut_directive)] @tag

(head_paragraph
  (head_directive) @directive
  (#eq? @directive "=head1")
  (content) @markup.heading.1)
(head_paragraph
  (head_directive) @directive
  (#eq? @directive "=head2")
  (content) @markup.heading.2)
(head_paragraph
  (head_directive) @directive
  (#eq? @directive "=head3")
  (content) @markup.heading.3)
(head_paragraph
  (head_directive) @directive
  (#eq? @directive "=head4")
  (content) @markup.heading.4)
(head_paragraph
  (head_directive) @directive
  (#eq? @directive "=head5")
  (content) @markup.heading.5)
(head_paragraph
  (head_directive) @directive
  (#eq? @directive "=head6")
  (content) @markup.heading.6)

(over_paragraph (content) @constant.numeric.integer)
(item_paragraph (content) @markup.list)
(encoding_paragraph (content) @string)

(verbatim_paragraph (content) @markup.raw)

(interior_sequence) @tag

(interior_sequence
  (sequence_letter) @letter
  (#eq? @letter "B")
  (content) @markup.bold)
(interior_sequence
  (sequence_letter) @letter
  (#eq? @letter "C")
  (content) @markup.raw)
(interior_sequence
  (sequence_letter) @letter
  (#eq? @letter "F")
  (content) @markup.italic)
(interior_sequence
  (sequence_letter) @letter
  (#eq? @letter "I")
  (content) @markup.italic)
(interior_sequence
  (sequence_letter) @letter
  (#eq? @letter "L")
  (content) @markup.link.url)
