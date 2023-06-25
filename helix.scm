(require-builtin helix/core/typable as helix.)
(require-builtin helix/core/static as helix.static.)
(require-builtin helix/core/keybindings as helix.keybindings.)

(provide set-theme-dracula
         set-theme-dracula__doc__
         set-theme-custom
         set-theme-custom__doc__
         theme-then-vsplit
         theme-then-vsplit__doc__
         custom-undo
         custom-undo__doc__
         lam
         lam__doc__
         delete-word-forward
         insert-string-at-selection
         highlight-to-matching-paren
         highlight-to-matching-paren__doc__
         delete-sexpr
         delete-sexpr__doc__
         run-expr
         run-highlight
         make-minor-mode!
         git-status

         reload-helix-scm
         static-format)

;;@doc
;; Sets the theme to be the dracula theme
(define (set-theme-dracula cx)
  (helix.theme cx (list "dracula") helix.PromptEvent::Validate))

(enqueue-callback! 'helix.static.format)
(enqueue-callback! 'set-theme-dracula)

;;@doc
;; Sets the theme to be the theme passed in
(define (set-theme-custom cx entered-theme)
  (helix.theme cx (list entered-theme) helix.PromptEvent::Validate))

;;@doc
;; Switch theme to the entered theme, then split the current file into
;; a vsplit
(define (theme-then-vsplit cx entered-theme)
  (set-theme-custom cx entered-theme)
  (helix.vsplit cx '() helix.PromptEvent::Validate))

;;@doc
;; Perform an undo
(define (custom-undo cx)
  (helix.static.undo cx))

;;@doc
;; Insert a lambda
(define (lam cx)
  (helix.static.insert_char cx #\λ)
  (helix.static.insert_mode cx))

;;@doc
;; Insert the string at the selection and go back into insert mode
(define (insert-string-at-selection cx str)
  (helix.static.insert_string cx str)
  (helix.static.insert_mode cx))

;;@doc
;; Delete the word forward
(define (delete-word-forward cx)
  (helix.static.delete_word_forward cx))

;;@doc
;; Registers a minor mode with the registered modifer and key map
;;
;; Examples:
;; ```scheme
;; (make-minor-mode! "+"
;;    (hash "P" ":lam"))
;; ```
(define (make-minor-mode! modifier bindings)
  (~> (hash "normal" (hash modifier bindings))
      (value->jsexpr-string)
      (helix.keybindings.set-keybindings!)))

(define-syntax minor-mode!
  (syntax-rules (=>)
    [(minor-mode! modifier (key => function))
     (make-minor-mode! modifier (minor-mode-cruncher (key => function)))]

    [(minor-mode! modifier (key => (function ...)))
     (make-minor-mode! modifier (minor-mode-cruncher (key => (function ...))))]

    [(minor-mode! modifier (key => function) remaining ...)
     (make-minor-mode! modifier (minor-mode-cruncher (key => function) remaining ...))]

    [(minor-mode! modifier (key => (function ...)) remaining ...)
     (make-minor-mode! modifier (minor-mode-cruncher (key => function) ... remaining ...))]))

(define-syntax minor-mode-cruncher
  (syntax-rules (=>)

    [(minor-mode-cruncher (key => (function ...)))
     (hash key (map (lambda (x) (string-append ":" (symbol->string x))) (quote (function ...))))]

    [(minor-mode-cruncher (key => function))
     (hash key (string-append ":" (symbol->string (quote function))))]

    [(minor-mode-cruncher (key => (function ...)) remaining ...)
     (hash-insert (minor-mode-cruncher remaining ...)
                  key
                  (map (lambda (x) (string-append ":" (symbol->string x))) (quote (function ...))))]

    [(minor-mode-cruncher (key => function) remaining ...)
     (hash-insert (minor-mode-cruncher remaining ...)
                  key
                  (string-append ":" (symbol->string (quote function))))]))

;;@doc
;; Highlight to the matching paren
(define (highlight-to-matching-paren cx)
  (helix.static.select_mode cx)
  (helix.static.match_brackets cx))

(define (run-expr cx)
  (define current-selection (helix.static.current_selection cx))
  (when (or (equal? "(" current-selection) (equal? ")" current-selection))
    (highlight-to-matching-paren cx)
    (helix.static.run-in-engine! cx (helix.static.current-highlighted-text! cx))
    (helix.static.normal_mode cx)))

(define (run-highlight cx)
  (helix.static.run-in-engine! cx (helix.static.current-highlighted-text! cx)))

;;@doc
;; Delete the s-expression matching this bracket
;; If the current selection is not on a bracket, this is a no-op
(define (delete-sexpr cx)
  (define current-selection (helix.static.current_selection cx))
  (when (or (equal? "(" current-selection) (equal? ")" current-selection))
    (highlight-to-matching-paren cx)
    (helix.static.delete_selection cx)))

; (minor-mode! "+" ("l" => lam)
;                  ("q" => (set-theme-dracula lam)))

(minor-mode! "P"
             ("l" => lam)
             ("p" => highlight-to-matching-paren)
             ("d" => delete-sexpr)
             ("r" => run-expr))

(make-minor-mode! "+" (hash "l" ":lam"))

(define (git-status cx)
  (helix.run-shell-command cx '("git" "status") helix.PromptEvent::Validate))

(minor-mode! "G" ("s" => git-status))

(define (reload-helix-scm cx)
  (helix.static.run-in-engine! cx
                               (string-append "(require \"" (helix.static.get-helix.scm-path) "\")")))
