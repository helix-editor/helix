(raw_blck
  (blob) @injection.shebang @injection.content)

(raw_blck
	lang: (ident) @injection.language
  (blob) @injection.content)

((comment)
	@injection.content
	(#set! injection.language "comment"))
