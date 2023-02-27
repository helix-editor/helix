(
 (s_string) @sql
 (#offset! @sql 0 2 0 -1)
)

(from_text
  (keyword_from_text)
  (keyword_json)
  (literal) @json
  (#offset! @json 0 3 0 -3)
)
