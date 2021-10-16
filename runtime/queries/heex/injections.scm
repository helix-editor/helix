((expression_value) @injection.content
 (#set! injection.language "elixir"))

; combined injection does not appear to work as expected,
; see: https://github.com/nvim-treesitter/nvim-treesitter/blob/58dd95f4a4db38a011c8f28564786c9d98b010c8/queries/heex/injections.scm#L1
; once combined injections works with this grammar, the following rules should be used instead:

; directives are standalone tags like <%= %>
; ((directive (expression_value) @injection.content)
;  (#set! injection.language "elixir")
;  (#set! injection.combined true))

; expressions live within HTML tags
;     <link href={ Routes.static_path(..) } />
; ((expression (expression_value) @injection.content)
;  (#set! injection.language "elixir"))
