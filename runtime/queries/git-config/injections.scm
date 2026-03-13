((comment) @injection.content
 (#set! injection.language "comment"))

; https://git-scm.com/docs/gitattributes#_defining_a_custom_hunk_header
; https://git-scm.com/docs/gitattributes#_customizing_word_diff
; e.g.
; ```
; [diff "tex"]
; 	xfuncname = "^(\\\\(sub)*section\\{.*)$"
; 	wordRegex = "\\\\[a-zA-Z]+|[{}]|\\\\.|[^\\{}[:space:]]+"
; ```
(variable
 (name) @_var (#any-of? @_var "xfuncname" "wordRegex")
 value: (string) @injection.content
 (#set! injection.language "regex"))
