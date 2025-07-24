(raw_blck
  (blob) @injection.shebang @injection.content)

(raw_blck
  lang: (ident) @_lang
  (blob) @injection.content
  (#set-lang-from-info-string! @_lang))
