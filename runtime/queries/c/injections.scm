((comment) @injection.content
 (#set! injection.language "comment"))

((preproc_arg) @injection.content
 (#set! injection.language "c")
 (#set! injection.include-children))

; Comments starting with `///`, `//!` are C++ style Doxygen comments, but
; comments starting with `////` are not.
((comment) @injection.content
 (#match? @injection.content "^[\t ]*(//!|///[^/])")
 (#set! injection.language "doxygen"))

; Comments starting with `/**`, `/*!` are C style Doxygen, but comments
; starting with `/***` are not.
((comment) @injection.content
 (#match? @injection.content "^[\t ]*(/\\*(!|\\*($|[^*])))")
 (#set! injection.language "doxygen"))
