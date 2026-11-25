; indents.scm --- Extempore / xtlang indentation
;
; Models the same indentation as `scmindent` (Dorai Sitaram) and the Extempore
; Emacs mode (extempore-mode's `extempore-indent-function`), so that on-disk
; files formatted with scmindent are a fixed point under both Helix and Emacs.
;
; The model has two cases:
;
;   1. "special forms" --- the keyword set below plus any `def...`-prefixed
;      symbol --- indent their body a fixed step (2 spaces) from the open paren,
;      like `defun`. This is the Emacs `extempore-indent-function` table:
;      def-prefix => defun, and the explicit `(put 'X 'extempore-indent-function
;      N)` entries.
;
;   2. every other form aligns its arguments under the column of the first
;      argument (operand alignment).
;
; Helix's indent engine expresses operand alignment with `@align` + an `@anchor`,
; and the fixed step with `@indent`. This targets the engine on Helix master
; (after 25.07.1), whose `@align` takes no "tail" scope. The structure here is adapted
; from Helix's bundled `scheme/indents.scm`, which cites the scmindent spec:
;   https://github.com/ds26gte/scmindent#how-subforms-are-indented
;
; The keyword regex is anchored exact-match (except the `def` prefix) so it
; tracks the Emacs `put`-table one-for-one. `set!` is intentionally absent (it
; is not in the Emacs table, so it operand-aligns, matching Emacs/scmindent).
; --- operand alignment (forms that are NOT special) ---
; If a list has 2 elements on the first line, align to the second element.
; Exclude literal first elements (different rule below) and the keyword set.
(list
  .
  (_) @_first
  .
  (_) @anchor
  (#same-line? @_first @anchor)
  (#not-kind-eq? @_first "boolean")
  (#not-kind-eq? @_first "character")
  (#not-kind-eq? @_first "string")
  (#not-kind-eq? @_first "number")
  (#not-match? @_first
    "^(def.*|let|letz|let[*]|letrec|let-syntax|letrec-syntax|lambda|case|dotimes|doloop|while|memzone|begin|delay|bind-func|bind-macro|bind-poly|bind-type|bind-val|bind-lib|bind-dylib|syntax-rules|syntax-case|call-with-input-file|with-input-from-file|with-input-from-port|call-with-output-file|with-output-to-file|with-output-to-port|call-with-values|dynamic-wind)$")) @align

; If the first element is itself a list alone on its line, align the outer list
; to it.
(list
  .
  (list) @anchor .) @align

(list
  .
  (list) @anchor
  .
  (_) @_second
  (#not-same-line? @anchor @_second)) @align

; If the first element is not a list and sits on a line by itself, indent the
; outer list one step (this is scmindent's "+1 space" reduced to a fixed step,
; exact when indent width is 2 and there is no space after the open paren).
(list
  .
  (_) @_first
  .
  (#not-kind-eq? @_first "boolean")
  (#not-kind-eq? @_first "character")
  (#not-kind-eq? @_first "string")
  (#not-kind-eq? @_first "number")
  (#not-match? @_first
    "^(def.*|let|letz|let[*]|letrec|let-syntax|letrec-syntax|lambda|case|dotimes|doloop|while|memzone|begin|delay|bind-func|bind-macro|bind-poly|bind-type|bind-val|bind-lib|bind-dylib|syntax-rules|syntax-case|call-with-input-file|with-input-from-file|with-input-from-port|call-with-output-file|with-output-to-file|with-output-to-port|call-with-values|dynamic-wind)$")) @indent

(list
  .
  (_) @_first
  .
  (_) @_second
  (#not-same-line? @_first @_second)
  (#not-kind-eq? @_first "boolean")
  (#not-kind-eq? @_first "character")
  (#not-kind-eq? @_first "string")
  (#not-kind-eq? @_first "number")
  (#not-match? @_first
    "^(def.*|let|letz|let[*]|letrec|let-syntax|letrec-syntax|lambda|case|dotimes|doloop|while|memzone|begin|delay|bind-func|bind-macro|bind-poly|bind-type|bind-val|bind-lib|bind-dylib|syntax-rules|syntax-case|call-with-input-file|with-input-from-file|with-input-from-port|call-with-output-file|with-output-to-file|with-output-to-port|call-with-values|dynamic-wind)$")) @indent

; If the first element is a literal, align the list to it.
(list
  .
  [
    (boolean)
    (character)
    (string)
    (number)
  ] @anchor) @align

; --- special forms: fixed-step body indent ---
; def-prefixed and the explicit keyword set indent their body one step.
(list
  .
  (symbol) @_first
  (#match? @_first
    "^(def.*|let|letz|let[*]|letrec|let-syntax|letrec-syntax|lambda|case|dotimes|doloop|while|memzone|begin|delay|bind-func|bind-macro|bind-poly|bind-type|bind-val|bind-lib|bind-dylib|syntax-rules|syntax-case|call-with-input-file|with-input-from-file|with-input-from-port|call-with-output-file|with-output-to-file|with-output-to-port|call-with-values|dynamic-wind)$")) @indent
