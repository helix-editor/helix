; directives are standalone tags like <%= .. %>
((directive (expression_value) @injection.content)
 (#set! injection.language "elixir")
 (#set! injection.combined))

; expressions live within HTML tags
;     <link href={ Routes.static_path(..) } />
((expression (expression_value) @injection.content)
 (#set! injection.language "elixir"))
