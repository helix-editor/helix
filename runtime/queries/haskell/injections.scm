((comment) @injection.content
 (#set! injection.language "comment"))

(quasiquote
 (quoter) @_quoter
 ((quasiquote_body) @injection.content
  (#match? @_quoter "(persistWith|persistLowerCase|persistUpperCase)")
  (#set! injection.language "haskell-persistent")
 )
)

(quasiquote
 (quoter) @injection.language
 (quasiquote_body) @injection.content)
