((comment) @injection.content
  (#set! injection.language "comment"))

((predicate
  name: (identifier) @_name
  parameters:
    (parameters
      (string (string_content) @injection.content)))
  (#any-of? @_name "match" "not-match")
  (#set! injection.language "regex"))
