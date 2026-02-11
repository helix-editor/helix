; TeX/LaTeX in Substance/Style labels
((latex) @injection.content
 (#set! injection.language "latex"))

; String-based labels are often TeX as well
((label_stmt (string) @injection.content)
 (#set! injection.language "latex"))
