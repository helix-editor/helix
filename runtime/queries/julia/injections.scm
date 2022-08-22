(
  (string_literal) @injection.content
  (#set! injection.language "markdown"))

(
  [
    (line_comment) 
    (block_comment)
  ] @injection.content
  (#set! injection.language "comment"))

(
  (prefixed_string_literal
    prefix: (identifier) @function.macro) @injection.content
  (#eq? @function.macro "re")
  (#set! injection.language "regex"))
