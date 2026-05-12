((s_string) @injection.content
 (#set! injection.language "sql"))

(from_text
  (keyword_from_text)
  (keyword_json)
  (literal) @injection.content
  (#set! injection.language "json"))
