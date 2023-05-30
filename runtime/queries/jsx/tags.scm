; Ecma based languages share many traits. Because of this we want to share as
; many queries as possible while avoiding nested inheritance that can make
; query behavior unpredictable due to unexpected precedence. To achieve that,
; some ecma related languages have "public" and "private" versions that share
; the same name, but the "private" version name starts with an underscore (with
; the exception of ecma, that works as the base "private" language). This
; allows the "private" versions to host the specific queries of the language
; excluding any "inherits" statement, in order to make them safe to be
; inherited by the "public" version of the same language and other languages
; as well.
; If you plan to add queries to this language, please consider adding them to
; any of the inherited languages listed below.

; inherits: _jsx,_javascript,ecma
