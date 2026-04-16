;; Note: The remaining typed commands will get appended
;; at runtime.

(require-builtin helix/core/typable as helix.)

(provide goto-column)

;;@doc
;; Move the cursor to the given character index within the same line
(define (goto-column col [extend #false])
  (helix.goto-column col extend))

(provide goto-line)

;;@doc
;; Move the cursor to the given line
(define (goto-line line [extend #false])
  (helix.goto-line line extend))
