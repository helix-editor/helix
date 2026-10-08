; ChordPro. The grammar is built from the ChordSketch repository
; (packages/tree-sitter-chordpro), which keeps a copy of this file in
; queries/helix/highlights.scm.

(comment) @comment.line

; Directives: {name} and {name: value}
(directive
  "{" @punctuation.bracket
  name: (directive_name) @keyword.directive
  "}" @punctuation.bracket)

(directive
  value: (directive_value) @string)

; Delegate blocks: {start_of_X} ... {end_of_X}
(block_start_directive
  "{" @punctuation.bracket
  name: (directive_name) @keyword.directive
  "}" @punctuation.bracket)

(block_end_directive
  "{" @punctuation.bracket
  name: (directive_name) @keyword.directive
  "}" @punctuation.bracket)

; Tablature, ABC and LilyPond bodies are a foreign notation carried
; verbatim, not ChordPro source.
(block_content) @markup.raw.block

; Chord annotations: [Am], [G/B]
(chord
  "[" @punctuation.bracket
  (chord_name) @constant
  "]" @punctuation.bracket)
