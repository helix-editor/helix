; The `comment` grammar is injected into the comments of most languages, so this
; query spell-checks comment prose across all of them. Only the prose `text` is
; checked; TODO-style tag names, their `(user)` mentions, and URLs are separate
; nodes and so are left unchecked.
"text" @spell
