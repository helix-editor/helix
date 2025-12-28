((comment) @injection.content
 (#set! injection.language "comment"))

(table
 (bare_key) @_table-name (#any-of? @_table-name "templates" "template-aliases")
 [(pair (_) ((string) @injection.content (#set! injection.language "jjtemplate"))) (comment)])

(table
 (bare_key) @_table-name (#any-of? @_table-name "revsets" "revset-aliases")
 [(pair (_) ((string) @injection.content (#set! injection.language "jjrevset"))) (comment)])

; Injections for aliases that contain inline scripts. (see `jj util exec --help`)
; This pattern currently relies on the language having the same name as its
; interpreter, which is often the case (sh, bash, python, fish, nu...)
; It also assumes the interpreter accepts the inline script with the "-c" flag.
(table
 (bare_key) @_table-name (#eq? @_table-name "aliases")
 (pair (_) (array .
  (string) @_util (#eq? @_util "\"util\"") . (string) @_exec (#eq? @_exec "\"exec\"") . (string) @_dd (#eq? @_dd "\"--\"") .
  (string) @injection.language .
  ; There are many possibilities to combine "-c" with other short flags, but by
  ; far the most common one should be the "-e" flag, which makes the script
  ; return early when an error occurs.
  (string) @_dc (#any-of? @_dc "\"-c\"" "\"-ce\"" "\"-ec\"") .
  (string) @injection.content)))
