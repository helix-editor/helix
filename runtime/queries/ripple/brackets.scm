; Bracket pairs for cursor navigation
(jsx_opening_element
  "<" @open
  ">" @close)

(jsx_closing_element
  "</" @open
  ">" @close)

(jsx_self_closing_element
  "<" @open
  "/>" @close)

("(" @open ")" @close)
("[" @open "]" @close)
("{" @open "}" @close)
