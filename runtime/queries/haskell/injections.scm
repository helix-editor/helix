((comment) @injection.content
 (#set! injection.language "comment"))

(quasiquote
 (quoter) @injection.language
 (quasiquote_body) @injection.content)
