; A highlight file for nvim-treesitter to use

[(pod_command)
 (command)
 (cut_command)] @keyword

(command_paragraph
  (command) @keyword
  (#eq? @keyword "=head1")
  (content) @markup.heading.1)

(command_paragraph
  (command) @keyword
  (#eq? @keyword "=head2")
  (content) @markup.heading.2)

(command_paragraph
  (command) @keyword
  (#eq? @keyword "=head3")
  (content) @markup.heading.3)

(command_paragraph
  (command) @keyword
  (#eq? @keyword "=head4")
  (content) @markup.heading.4)

(command_paragraph
  (command) @keyword
  (#eq? @keyword "=head5")
  (content) @markup.heading.5)

(command_paragraph
  (command) @keyword
  (#eq? @keyword "=head6")
  (content) @markup.heading.6)

(command_paragraph
  (command) @keyword
  (#match? @keyword "^=over")
  (content) @constant.numeric)

(command_paragraph
  (command) @keyword
  (#match? @keyword "^=item")
  (content) @markup)

(command_paragraph
  (command) @keyword
  (#match? @keyword "^=encoding")
  (content) @string.special)

(command_paragraph
  (command) @keyword
  (#not-match? @keyword "^=(head|over|item|encoding)")
  (content) @string)

(verbatim_paragraph (content) @markup.raw)

(interior_sequence
  (sequence_letter) @constant.character
  ["<" ">"] @punctuation.delimiter
)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "B")
  (content) @markup.bold)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "C")
  (content) @markup.literal)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "F")
  (content) @markup.underline @string.special)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "I")
  (content) @markup.bold)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "L")
  (content) @markup.link.url)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "X")
  (content) @markup.reference)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "E")
  (content) @string.special.escape)
