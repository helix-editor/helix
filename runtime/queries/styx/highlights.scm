(line_comment) @comment.line
(doc_comment) @comment.line.documentation

(escape_sequence) @constant.character.escape

(bare_scalar) @string
(quoted_scalar) @string
(raw_scalar) @string
(heredoc) @string

; Heredoc language hint (metadata)
(heredoc_lang) @label

(unit) @constant.builtin

; Tags - styled same as unit since @ is the tag sigil
(tag) @constant.builtin

; Attributes - key in attribute syntax
; Use @keyword or @punctuation.special to make > stand out
(attribute
  key: (bare_scalar) @variable.other.member
  ">" @keyword)

; Keys in entries - any scalar in the key position (overrides @string above)
(entry
  key: (expr
    payload: (scalar (_) @variable.other.member)))

; Sequence items are values, not keys (must come AFTER entry key rule to override)
(sequence
  (expr
    payload: (scalar (_) @string)))

[
  "{"
  "}"
  "("
  ")"
] @punctuation.bracket

"," @punctuation.delimiter
