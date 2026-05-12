; Bird track marker
(bird_line ">" @punctuation.special)

; LaTeX delimiters
(latex_begin) @keyword.directive
(latex_end) @keyword.directive

; Highlight LaTeX comments like comments
(latex_comment) @comment

; Markdown delimiters
(markdown_begin) @keyword.directive
(markdown_end) @keyword.directive

; Normal prose is not highlighted. Haskell code will be
; highlighted by the injected Haskell grammar
