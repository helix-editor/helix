; Inject Haskell parser into bird-style code lines
((bird_line
  (haskell_code) @injection.content)
 (#set! injection.language "haskell"))

; Inject Haskell parser into LaTeX code blocks
((latex_code_line
  (haskell_code) @injection.content)
 (#set! injection.language "haskell"))

; Inject Haskell parser into Markdown code blocks
((markdown_code_line
  (haskell_code) @injection.content)
 (#set! injection.language "haskell"))
