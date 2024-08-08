; From <https://github.com/IndianBoy42/tree-sitter-just/blob/6c2f018ab1d90946c0ce029bb2f7d57f56895dff/queries-flavored/helix/injections.scm>
;
; Specify nested languages that live within a `justfile`

; ================ Always applicable ================

((comment) @injection.content
  (#set! injection.language "comment"))

; Highlight the RHS of `=~` as regex
((regex_literal
  (_) @injection.content)
  (#set! injection.language "regex"))

; ================ Global defaults ================

; Default everything to be bash
(recipe_body
  !shebang
  (#set! injection.language "bash")
  (#set! injection.include-children)) @injection.content

(external_command
  (command_body) @injection.content
  (#set! injection.language "bash"))

; ================ Global language specified ================
; Global language is set with something like one of the following:
;
;    set shell := ["bash", "-c", ...]
;    set shell := ["pwsh.exe"]
;
; We can extract the first item of the array, but we can't extract the language
; name from the string with something like regex. So instead we special case
; two things: powershell, which is likely to come with a `.exe` attachment that
; we need to strip, and everything else which hopefully has no extension. We
; separate this with a `#match?`.
;
; Unfortunately, there also isn't a way to allow arbitrary nesting or
; alternatively set "global" capture variables. So we can set this for item-
; level external commands, but not for e.g. external commands within an
; expression without getting _really_ annoying. Should at least look fine since
; they default to bash. Limitations...
; See https://github.com/tree-sitter/tree-sitter/issues/880 for more on that.

(source_file
  (setting "shell" ":=" "[" (string) @_langstr
    (#match? @_langstr ".*(powershell|pwsh|cmd).*")
    (#set! injection.language "powershell"))
  [
    (recipe
      (recipe_body
        !shebang
        (#set! injection.include-children)) @injection.content)

    (assignment
      (expression
        (value
          (external_command
            (command_body) @injection.content))))
  ])

(source_file
  (setting "shell" ":=" "[" (string) @injection.language
    (#not-match? @injection.language ".*(powershell|pwsh|cmd).*"))
  [
    (recipe
      (recipe_body
        !shebang
        (#set! injection.include-children)) @injection.content)

    (assignment
      (expression
        (value
          (external_command
            (command_body) @injection.content))))
  ])

; ================ Recipe language specified - Helix only ================

; Set highlighting for recipes that specify a language using builtin shebang matching
(recipe_body
  (shebang) @injection.shebang
  (#set! injection.include-children)) @injection.content
