((expression_value) @injection.content
 (#set! injection.language "elixir"))

; combined injection does not appear to work as expected yet
; once combined injections work, these rules should be used instead of the above

; directives are standalone tags like <%= %>
; ((directive (expression_value) @injection.content)
;  (#set! injection.language "elixir")
;  (#set! injection.combined true))

; expressions live within HTML tags
;     <link href={ Routes.static_path(..) } />
; ((expression (expression_value) @injection.content)
;  (#set! injection.language "elixir"))
