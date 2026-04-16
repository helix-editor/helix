(require "helix/editor.scm")
(require "helix/misc.scm")
(require-builtin helix/core/text as text.)
(require "steel/sync")

(provide eval-buffer
         evalp
         running-on-main-thread?
         hx.with-context
         hx.block-on-task)

(define (get-document-as-slice)
  (let* ([focus (editor-focus)]
         [focus-doc-id (editor->doc-id focus)])
    (text.rope->string (editor->text focus-doc-id))))

;;@doc
;; Eval the current buffer, morally equivalent to load-buffer!
(define (eval-buffer)
  (eval-string (get-document-as-slice)))

;;@doc
;; Eval prompt
(define (evalp)
  (push-component! (prompt "" (lambda (expr) (set-status! (eval-string expr))))))

;;@doc
;; Check what the main thread id is, compare to the main thread
(define (running-on-main-thread?)
  (= (current-thread-id) *helix.id*))

;;@doc
;; If running on the main thread already, just do nothing.
;; Check the ID of the engine, and if we're already on the
;; main thread, just continue as is - i.e. just block. This does
;; not block on the function if this is running on another thread.
;;
;; ```scheme
;; (hx.with-context thunk)
;; ```
;; thunk : (-> any?) ;; Function that has no arguments
;;
;; # Examples
;; ```scheme
;; (spawn-native-thread
;;   (lambda ()
;;     (hx.with-context (lambda () (theme "nord")))))
;; ```
(define (hx.with-context thunk)
  (if (running-on-main-thread?)
      (thunk)
      (begin
        (define task (task #f))
        ;; Send on the main thread
        (acquire-context-lock thunk task)
        task)))

;;@doc
;; Block on the given function.
;; ```scheme
;; (hx.block-on-task thunk)
;; ```
;; thunk : (-> any?) ;; Function that has no arguments
;;
;; # Examples
;; ```scheme
;; (define thread
;;   (spawn-native-thread
;;     (lambda ()
;;       (hx.block-on-task (lambda () (theme "nord") 10)))))
;;
;; ;; Some time later, in a different context - if done at the same time,
;; ;; this will deadline, since the join depends on the callback previously
;; ;; executing.
;; (equal? (thread-join! thread) 10) ;; => #true
;; ```
(define (hx.block-on-task thunk)
  (if (running-on-main-thread?)
      (thunk)
      (block-on-task (hx.with-context thunk))))
