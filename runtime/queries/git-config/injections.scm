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

((section_header (section_name) @markup.heading)
 (#eq? @markup.heading "alias")
 (variable (name)
  value: (string) @injection.content
   ; ASK: some way to not highlight the bang as `sh`?
   ; ASK: add `(?s)` (dot-all) flag?
   (#match? @injection.content "(^\"!.*\"$)|(^!)")
  (#set! injection.language "bash"))
)

(variable
 ; ASK: is this overly general?
 (name) @_var (#eq? @_var "helper")
 value: (string) @injection.content
  (#match? @injection.content "(^\"!.*\"$)|(^!)")
 (#set! injection.language "bash"))

; TODO: missing `*.cmd` sections
