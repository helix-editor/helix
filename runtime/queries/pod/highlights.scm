; A highlight file for nvim-treesitter to use

[(pod_command)
 (command)
 (cut_command)] @keyword

(command_paragraph
  (command) @keyword
  (#match? @keyword "^=head")
  (content) @markup.title)

(command_paragraph
  (command) @keyword
  (#match? @keyword "^=over")
  (content) @number)

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
  (sequence_letter) @character
  ["<" ">"] @punctuation.delimiter
)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "B")
  (content) @markup.strong)

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
  (content) @markup.emphasis)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "L")
  (content) @markup.uri)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "X")
  (content) @markup.reference)

(interior_sequence
  (sequence_letter) @character
  (#eq? @character "E")
  (content) @string.escape)
