((comment) @injection.content
 (#set! injection.language "comment"))

((predicate
   (predicate_name) @_predicate
   (string) @injection.content)
 (#eq? @_predicate "#match?")
 (#set! injection.language "regex"))
