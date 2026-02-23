(require-builtin helix/core/configuration as helix.)

;;@doc
;; Set a configuration option by key name.
(provide set-option!)
(define set-option! helix.set-option!)

(provide statusline)

;;@doc
;; Configuration of the statusline elements.
;; The following status line elements can be configured:
;;
;; Key	                        Description
;; -------------------------------------------------------------------------------------------
;; mode	                        The current editor mode (mode.normal/mode.insert/mode.select)
;; spinner	                    A progress spinner indicating LSP activity
;; file-name	                The path/name of the opened file
;; file-absolute-path	        The absolute path/name of the opened file
;; file-base-name	            The basename of the opened file
;; file-modification-indicator	The indicator to show whether the file is modified (a [+] appears when there are unsaved changes)
;; file-encoding	            The encoding of the opened file if it differs from UTF-8
;; file-line-ending	            The file line endings (CRLF or LF)
;; file-indent-style	        The file indentation style
;; read-only-indicator	        An indicator that shows [readonly] when a file cannot be written
;; total-line-numbers	        The total line numbers of the opened file
;; file-type	                The type of the opened file
;; diagnostics	                The number of warnings and/or errors
;; workspace-diagnostics	    The number of warnings and/or errors on workspace
;; selections	                The primary selection index out of the number of active selections
;; primary-selection-length	    The number of characters currently in primary selection
;; position	                    The cursor position
;; position-percentage	        The cursor position as a percentage of the total number of lines
;; separator	                The string defined in editor.statusline.separator (defaults to "│")
;; spacer	                    Inserts a space between elements (multiple/contiguous spacers may be specified)
;; version-control	            The current branch name or detached commit hash of the opened workspace
;; register	                    The current selected register
(define (statusline
         #:left
         [left
          (list "mode" "spinner" "file-name" "read-only-indicator" "file-modification-indicator")]
         #:center [center '()]
         #:right [right (list "diagnostics" "selections" "register" "position" "file-encoding")]
         #:separator [separator "|"]
         #:mode-normal [mode-normal "NOR"]
         #:mode-insert [mode-insert "INS"]
         #:mode-select [mode-select "SEL"]
         #:diagnostics [diagnostics (list "warning" "error")]
         #:workspace-diagnostics [workspace-diagnostics (list "warning" "error")])
  (helix.statusline (hash 'left
                          left
                          'center
                          center
                          'right
                          right
                          'separator
                          separator
                          'mode-normal
                          mode-normal
                          'mode-insert
                          mode-insert
                          'mode-select
                          mode-select
                          'diagnostics
                          diagnostics
                          'workspace-diagnostics
                          workspace-diagnostics)))

(provide indent-heuristic)
;;@doc
;; Which indent heuristic to use when a new line is inserted
;; Defaults to `"hybrid"`
;; Valid options are:
;; * "simple"
;; * "tree-sitter"
;; * "hybrid"
(define (indent-heuristic kind)
  (set-option! 'indent-heuristic kind))

(provide atomic-save)

;;@doc
;; Whether to use atomic operations to write documents to disk.
;; This prevents data loss if the editor is interrupted while writing the file, but may
;; confuse some file watching/hot reloading programs. Defaults to `#true`.
(define (atomic-save bool-opt)
  (set-option! 'atomic-save bool-opt))

(provide lsp)

;;@doc
;; Blanket LSP configuration
;; The options are provided in a hashmap, and provided options will be merged
;; with the defaults. The options are as follows:
;;
;; Enables LSP
;; * enable: bool
;;
;; Display LSP messagess from $/progress below statusline
;; * display-progress-messages: bool
;;
;; Display LSP messages from window/showMessage below statusline
;; * display-messages: bool
;;
;; Enable automatic pop up of signature help (parameter hints)
;; * auto-signature-help: bool
;;
;; Display docs under signature help popup
;; * display-signature-help-docs: bool
;;
;; Display inlay hints
;; * display-inlay-hints: bool
;;
;; Maximum displayed length of inlay hints (excluding the added trailing `…`).
;; If it's `None`, there's no limit
;; * inlay-hints-length-limit: Option<NonZeroU8>
;;
;; Display document color swatches
;; * display-color-swatches: bool
;;
;; Whether to enable snippet support
;; * snippets: bool
;;
;; Whether to include declaration in the goto reference query
;; * goto_reference_include_declaration: bool
;;
;;```scheme
;; (lsp (hash 'display-inlay-hints #t))
;;```
;;
;; The defaults shown from the rust side are as follows:
;; ```rust
;;         LspConfig {
;;            enable: true,
;;            display_progress_messages: false,
;;            display_messages: true,
;;            auto_signature_help: true,
;;            display_signature_help_docs: true,
;;            display_inlay_hints: false,
;;            inlay_hints_length_limit: None,
;;            snippets: true,
;;            goto_reference_include_declaration: true,
;;            display_color_swatches: true,
;;        }
;;
;; ```
(define lsp helix.lsp)

(provide search)

;;@doc
;; Search configuration
;; Accepts two keywords, #:smart-case and #:wrap-around, both default to true.
;;
;; ```scheme
;; (search #:smart-case #t #:wrap-around #t)
;; (search #:smart-case #f #:wrap-around #f)
;; ```
(define (search #:smart-case [smart-case #t] #:wrap-around [wrap-around #true])
  (helix.search smart-case wrap-around))

(provide auto-pairs)

;;@doc
;; Automatic insertion of pairs to parentheses, brackets,
;; etc. Optionally, this can be a list of pairs to specify a
;; global list of characters to pair, or a hashmap of character to character.
;; Defaults to true.
;;
;; ```scheme
;; (auto-pairs #f)
;; (auto-pairs #t)
;; (auto-pairs (list '(#\{ . #\})))
;; (auto-pairs (list '(#\{ #\})))
;; (auto-pairs (list (cons #\{ #\})))
;; (auto-pairs (hash #\{ #\}))
;; ```
(define (auto-pairs bool-or-map-or-pairs)
  (when (hash? bool-or-map-or-pairs)
    (helix.auto-pairs (helix.auto-pairs-map bool-or-map-or-pairs))
    (helix.#%editor-auto-pairs (helix.auto-pairs-map bool-or-map-or-pairs)))

  (when (bool? bool-or-map-or-pairs)
    (helix.auto-pairs (helix.auto-pairs-default bool-or-map-or-pairs))
    (helix.#%editor-auto-pairs (helix.auto-pairs-default bool-or-map-or-pairs)))

  (when (list? bool-or-map-or-pairs)
    (helix.auto-pairs (helix.auto-pairs-map (#%prim.transduce bool-or-map-or-pairs (into-hashmap))))
    (helix.#%editor-auto-pairs (helix.auto-pairs-map (#%prim.transduce bool-or-map-or-pairs
                                                                       (into-hashmap))))))

(provide continue-comments)

;;@doc
;; Whether comments should be continued.
(define (continue-comments bool)
  (set-option! 'continue-comments bool))

(provide popup-border)

;;@doc
;; Set the popup border.
;; Valid options are:
;; * "none"
;; * "all"
;; * "popup"
;; * "menu"
(define (popup-border option)
  (set-option! 'popup-border option))

(provide register-lsp-notification-handler)

;;@doc
;; Register a callback to be called on LSP notifications sent from the server -> client
;; that aren't currently handled by Helix as a built in.
;;
;; ```scheme
;; (register-lsp-notification-handler lsp-name event-name handler)
;; ```
;;
;; * lsp-name : string?
;; * event-name : string?
;; * function : (-> hash? any?) ;; Function where the first argument is the parameters
;;
;; # Examples
;; ```
;; (register-lsp-notification-handler "dart"
;;                                    "dart/textDocument/publishClosingLabels"
;;                                    (lambda (args) (displayln args)))
;; ```
(define register-lsp-notification-handler helix.register-lsp-notification-handler)

(provide register-lsp-call-handler)

;;@doc
;; Register a callback to be called on LSP calls sent from the server -> client
;; that aren't currently handled by Helix as a built in.
;;
;; ```scheme
;; (register-lsp-call-handler lsp-name event-name handler)
;; ```
;;
;; * lsp-name : string?
;; * event-name : string?
;; * function : (-> hash? any?) ;; Function where the first argument is the parameters
;;
;; # Examples
;; ```
;; (register-lsp-call-handler "dart"
;;                                    "dart/textDocument/publishClosingLabels"
;;                                    (lambda (call-id args) (displayln args)))
;; ```
(define register-lsp-call-handler helix.register-lsp-call-handler)

(provide define-lsp)
;;@doc
;; Syntax:
;;
;; Registers an lsp configuration. This is a thin wrapper around passing
;; a hashmap manually to `set-lsp-config!`, and has a slightly more elegant
;; API.
;;
;; Examples:
;; ```scheme
;; (define-lsp "steel-language-server" (command steel-language-server) (args '()))
;; (define-lsp "rust-analyzer" (config (experimental (hash 'testExplorer #t 'runnables '("cargo")))))
;; (define-lsp "tinymist" (config (exportPdf "onType") (outputPath "$root/$dir/$name")))
;; ```
(define-syntax define-lsp
  (syntax-rules (#%crunch #%name #%conf)
    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...))
     (set-lsp-config! name
                      (hash-insert conf
                                   (quote key)
                                   (transduce (list (list (quote inner-key) value) ...)
                                              (into-hashmap))))]

    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-lsp #%crunch
                 #%name
                 name
                 #%conf
                 (hash-insert conf
                              (quote key)
                              (transduce (list (list (quote inner-key) value) ...) (into-hashmap)))
                 remaining ...)]

    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key value))
     (set-lsp-config! name (hash-insert conf (quote key) value))]

    [(_ #%crunch #%name name #%conf conf (key value) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-lsp #%crunch #%name name #%conf (hash-insert conf (quote key) value) remaining ...)]

    [(_ name (key value ...) ...)
     (define-lsp #%crunch #%name name #%conf (hash "name" name) (key value ...) ...)]

    [(_ name (key value)) (define-lsp #%crunch #%name name #%conf (hash "name" name) (key value))]

    [(_ name (key value) ...)
     (define-lsp #%crunch #%name name #%conf (hash "name" name) (key value) ...)]))

(provide define-language)

;;@doc
;; Syntax:
;;
;; Defines a language configuration.
;; This is a thin wrapper around calling `update-language-config!` with a hash
;; of arguments, and has a slightly more elegant syntax.
;;
;; ```scheme
;; (define-language "scheme"
;;                 (formatter (command "raco") (args '("fmt" "-i")))
;;                 (auto-format #true)
;;                 (language-servers '("steel-language-server")))
;;
;; ```
(define-syntax define-language
  (syntax-rules (#%crunch #%name #%conf)

    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...))
     (update-language-config! name
                              (hash-insert conf
                                           (quote key)
                                           (transduce (list (list (quote inner-key) value) ...)
                                                      (into-hashmap))))]

    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-language #%crunch
                      #%name
                      name
                      #%conf
                      (hash-insert conf
                                   (quote key)
                                   (transduce (list (list (quote inner-key) value) ...)
                                              (into-hashmap)))
                      remaining ...)]

    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key value))
     (update-language-config! name (hash-insert conf (quote key) value))]

    [(_ #%crunch #%name name #%conf conf (key value) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-language #%crunch #%name name #%conf (hash-insert conf (quote key) value) remaining ...)]

    [(_ name (key value ...) ...)
     (define-language #%crunch #%name name #%conf (hash "name" name) (key value ...) ...)]

    [(_ name (key value)) (language #%crunch #%name name #%conf (hash "name" name) (key value))]

    [(_ name (key value) ...)
     (define-language #%crunch #%name name #%conf (hash "name" name) (key value) ...)]))

(provide cursor-shape)
;;@doc
;; Shape for cursor in each mode
;;
;; (cursor-shape #:normal (normal 'block)
;;               #:select (select 'block)
;;               #:insert (insert 'block))
;;
;; # Examples
;;
;; ```scheme
;; (cursor-shape #:normal 'block #:select 'underline #:insert 'bar)
;; ```
(define (cursor-shape #:normal (normal 'block) #:select (select 'block) #:insert (insert 'block))
  (define cursor-shape-config (helix.raw-cursor-shape))
  (helix.raw-cursor-shape-set! cursor-shape-config 'normal normal)
  (helix.raw-cursor-shape-set! cursor-shape-config 'select select)
  (helix.raw-cursor-shape-set! cursor-shape-config 'insert insert)
  (helix.#%raw-cursor-shape cursor-shape-config))

(provide refresh-all-language-configs!)
(define refresh-all-language-configs! helix.refresh-all-language-configs!)

(provide update-configuration!)
(define update-configuration! helix.update-configuration!)

(provide get-config-option-value)
(define get-config-option-value helix.get-config-option-value)

(provide set-configuration-for-file!)
(define set-configuration-for-file! helix.set-configuration-for-file!)

(provide get-lsp-config)

;;@doc
;; Get the lsp configuration for a language server.
;;
;; Returns a hashmap which can be passed to `set-lsp-config!`
(define get-lsp-config helix.get-lsp-config)

(provide set-lsp-config!)
;;@doc
;; Sets the language server config for a specific language server.
;;
;; ```scheme
;; (set-lsp-config! lsp config)
;; ```
;; * lsp : string?
;; * config: hash?
;;
;; This will overlay the existing configuration, much like the existing
;; toml definition does.
;;
;; Available options for the config hash are:
;; ```scheme
;; (hash "command" "<command>"
;;       "args" (list "args" ...)
;;       "environment" (hash "ENV" "VAR" ...)
;;       "config" (hash ...)
;;       "timeout" 100 ;; number
;;       "required-root-patterns" (listof "pattern" ...))
;;
;; ```
;;
;; # Examples
;; ```
;; (set-lsp-config! "jdtls"
;;    (hash "args" (list "-data" "/home/matt/code/java-scratch/workspace")))
;; ```
(define set-lsp-config! helix.set-lsp-config!)

(provide update-language-config!)
(define (update-language-config! lsp config)
  (helix.update-language-config! lsp config)
  (refresh-all-language-configs!))

(provide get-keybindings)
(define get-keybindings helix.get-keybindings)

(provide file-picker-kw)
;;@doc
;; Sets the configuration for the file picker using keywords.
;;
;; ```scheme
;; (file-picker-kw #:hidden #t
;;                 #:follow-symlinks #t
;;                 #:deduplicate-links #t
;;                 #:parents #t
;;                 #:ignore #t
;;                 #:git-ignore #t
;;                 #:git-exclude #t
;;                 #:git-global #t
;;                 #:max-depth #f) ;; Expects either #f or an int?
;; ```
;; By default, max depth is `#f` while everything else is an int?
;;
;; To use this, call this in your `init.scm` or `helix.scm`:
;;
;; # Examples
;; ```scheme
;; (file-picker-kw #:hidden #f)
;; ```
(define (file-picker-kw #:hidden [hidden #t]
                        #:follow-symlinks [follow-symlinks #t]
                        #:deduplicate-links [deduplicate-links #t]
                        #:parents [parents #t]
                        #:ignore [ignore #t]
                        #:git-ignore [git-ignore #t]
                        #:git-global [git-global #t]
                        #:git-exclude [git-exclude #t]
                        #:max-depth [max-depth #f])

  (define picker (helix.raw-file-picker))
  (unless hidden
    (helix.fp-hidden picker hidden))
  (unless follow-symlinks
    (helix.fp-follow-symlinks picker follow-symlinks))
  (unless deduplicate-links
    (helix.fp-deduplicate-links picker deduplicate-links))
  (unless parents
    (helix.fp-parents picker parents))
  (unless ignore
    (helix.fp-ignore picker ignore))
  (unless git-ignore
    (helix.fp-git-ignore picker git-ignore))
  (unless git-global
    (helix.fp-git-global picker git-global))
  (unless git-exclude
    (helix.fp-git-exclude picker git-exclude))
  (when max-depth
    (helix.fp-max-depth picker max-depth))
  (helix.register-file-picker picker))

(provide file-picker)
;;@doc
;; Sets the configuration for the file picker using var args.
;;
;; ```scheme
;; (file-picker . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> FilePickerConfiguration? bool?)
;; ```
;;
;; These other functions in this module which follow this behavior are all
;; prefixed `fp-`, and include:
;;
;; * fp-hidden
;; * fp-follow-symlinks
;; * fp-deduplicate-links
;; * fp-parents
;; * fp-ignore
;; * fp-git-ignore
;; * fp-git-global
;; * fp-git-exclude
;; * fp-max-depth
;;
;; By default, max depth is `#f` while everything else is an int?
;;
;; To use this, call this in your `init.scm` or `helix.scm`:
;;
;; # Examples
;; ```scheme
;; (file-picker (fp-hidden #f) (fp-parents #f))
;; ```
(define (file-picker . args)
  (helix.register-file-picker
   (foldl (lambda (func config) (func config)) (helix.raw-file-picker) args)))

(provide soft-wrap-kw)
;;@doc
;; Sets the configuration for soft wrap using keyword args.
;;
;; ```scheme
;; (soft-wrap-kw #:enable #f
;;               #:max-wrap 20
;;               #:max-indent-retain 40
;;               #:wrap-indicator "↪"
;;               #:wrap-at-text-width #f)
;; ```
;;
;; The options are as follows:
;;
;; * #:enable:
;;   Soft wrap lines that exceed viewport width. Default to off
;; * #:max-wrap:
;;   Maximum space left free at the end of the line.
;;   This space is used to wrap text at word boundaries. If that is not possible within this limit
;;   the word is simply split at the end of the line.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;;
;;   Default to 20
;; * #:max-indent-retain
;;   Maximum number of indentation that can be carried over from the previous line when softwrapping.
;;   If a line is indented further then this limit it is rendered at the start of the viewport instead.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;;
;;   Default to 40
;; * #:wrap-indicator
;;   Indicator placed at the beginning of softwrapped lines
;;
;;   Defaults to ↪
;; * #:wrap-at-text-width
;;   Softwrap at `text_width` instead of viewport width if it is shorter
;;
;; # Examples
;; ```scheme
;; (soft-wrap-kw #:sw-enable #t)
;; ```
(define (soft-wrap-kw #:enable [enable #f]
                      #:max-wrap [max-wrap 20]
                      #:max-indent-retain [max-indent-retain 40]
                      #:wrap-indicator [wrap-indicator 4]
                      #:wrap-at-text-width [wrap-at-text-width #f])
  (define sw (helix.raw-soft-wrap))
  (helix.sw-enable sw enable)
  (helix.sw-max-wrap sw max-wrap)
  (helix.sw-max-indent-retain sw max-indent-retain)
  (helix.sw-wrap-indicator sw wrap-indicator)
  (helix.sw-wrap-at-text-width sw wrap-at-text-width)
  (helix.register-soft-wrap sw))

(provide soft-wrap)
;;@doc
;; Sets the configuration for soft wrap using var args.
;;
;; ```scheme
;; (soft-wrap . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> SoftWrapConfiguration? bool?)
;; ```
;; The options are as follows:
;;
;; * sw-enable:
;;   Soft wrap lines that exceed viewport width. Default to off
;; * sw-max-wrap:
;;   Maximum space left free at the end of the line.
;;   This space is used to wrap text at word boundaries. If that is not possible within this limit
;;   the word is simply split at the end of the line.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;;
;;   Default to 20
;; * sw-max-indent-retain
;;   Maximum number of indentation that can be carried over from the previous line when softwrapping.
;;   If a line is indented further then this limit it is rendered at the start of the viewport instead.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;;
;;   Default to 40
;; * sw-wrap-indicator
;;   Indicator placed at the beginning of softwrapped lines
;;
;;   Defaults to ↪
;; * sw-wrap-at-text-width
;;   Softwrap at `text_width` instead of viewport width if it is shorter
;;
;; # Examples
;; ```scheme
;; (soft-wrap (sw-enable #t))
;; ```
(define (soft-wrap . args)
  (helix.register-soft-wrap (foldl (lambda (func config) (func config)) (helix.raw-soft-wrap) args)))

(provide whitespace)
;;@doc
;; Sets the configuration for whitespace using var args.
;;
;; ```scheme
;; (whitespace . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> WhitespaceConfiguration? bool?)
;; ```
;; The options are as follows:
;;
;; * ws-visible:
;;   Show all visible whitespace, defaults to false
;; * ws-render:
;;   manually disable or enable characters
;;   render options (specified in hashmap):
;;```scheme
;;   (hash
;;     'space #f
;;     'nbsp #f
;;     'nnbsp #f
;;     'tab #f
;;     'newline #f)
;;```
;; * ws-chars:
;;   manually set visible whitespace characters with a hashmap
;;   character options (specified in hashmap):
;;```scheme
;;   (hash
;;     'space #\·
;;     'nbsp #\⍽
;;     'nnbsp #\␣
;;     'tab #\→
;;     'newline #\⏎
;;     ; Tabs will look like "→···" (depending on tab width)
;;     'tabpad #\·)
;;```
;; # Examples
;; ```scheme
;; (whitespace (ws-visible #t) (ws-chars (hash 'space #\·)) (ws-render (hash 'tab #f)))
;; ```
(define (whitespace . args)
  (helix.register-whitespace
   (foldl (lambda (func config) (func config)) (helix.raw-whitespace) args)))

(provide indent-guides)
;;@doc
;; Sets the configuration for indent-guides using args
;;
;; ```scheme
;; (indent-guides . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> IndentGuidesConfig? bool?)
;; ```
;; The options are as follows:
;;
;; * ig-render:
;;   Show indent guides, defaults to false
;; * ig-character:
;;   character used for indent guides, defaults to "╎"
;; * ig-skip-levels:
;;   amount of levels to skip, defaults to 1
;;
;; # Examples
;; ```scheme
;; (indent-guides (ig-render #t) (ig-character #\|) (ig-skip-levels 1))
;; ```
(define (indent-guides . args)
  (helix.register-indent-guides
   (foldl (lambda (func config) (func config)) (helix.raw-indent-guides) args)))

(provide scrolloff)

;;@doc
;; Padding to keep between the edge of the screen and the cursor when scrolling. Defaults to 5.
(define scrolloff helix.scrolloff)

(provide scroll_lines)

;;@doc
;; Number of lines to scroll at once. Defaults to 3
(define scroll_lines helix.scroll_lines)

(provide mouse)

;;@doc
;; Mouse support. Defaults to true.
(define (mouse opt)
  (helix.mouse opt)
  (helix.#%update-configuration *helix.config*))

(provide shell)

;;@doc
;; Shell to use for shell commands. Defaults to ["cmd", "/C"] on Windows and ["sh", "-c"] otherwise.
(define shell helix.shell)

(provide jump-label-alphabet)

;;@doc
;; The characters that are used to generate two character jump labels.
;; Characters at the start of the alphabet are used first. Defaults to "abcdefghijklmnopqrstuvwxyz"
(define jump-label-alphabet helix.jump-label-alphabet)

(provide line-number)

;;@doc
;; Line number mode. Defaults to 'absolute, set to 'relative for relative line numbers
(define line-number helix.line-number)

(provide cursorline)

;;@doc
;; Highlight the lines cursors are currently on. Defaults to false
(define cursorline helix.cursorline)

(provide cursorcolumn)

;;@doc
;; Highlight the columns cursors are currently on. Defaults to false
(define cursorcolumn helix.cursorcolumn)

(provide middle-click-paste)

;;@doc
;; Middle click paste support. Defaults to true
(define middle-click-paste helix.middle-click-paste)

(provide auto-completion)

;;@doc
;; Automatic auto-completion, automatically pop up without user trigger. Defaults to true.
(define auto-completion helix.auto-completion)

(provide auto-format)

;;@doc
;; Automatic formatting on save. Defaults to true
(define auto-format helix.auto-format)

(provide auto-save)

;;@doc
;; Automatic save on focus lost and/or after delay.
;; Time delay in milliseconds since last edit after which auto save timer triggers.
;; Time delay defaults to false with 3000ms delay. Focus lost defaults to false.
(define auto-save helix.auto-save)

(provide auto-save-after-delay-enable)
;;@doc
;; Enables auto save after delay. Default is false.
(define auto-save-after-delay-enable helix.auto-save-after-delay-enable)

(provide text-width)

;;@doc
;; Set a global text_width
(define text-width helix.text-width)

(provide idle-timeout)

;;@doc
;; Time in milliseconds since last keypress before idle timers trigger.
;; Used for various UI timeouts. Defaults to 250ms.
(define idle-timeout helix.idle-timeout)

(provide completion-timeout)
;;@doc
;; Time in milliseconds after typing a word character before auto completions
;; are shown, set to 5 for instant. Defaults to 250ms.
(define completion-timeout helix.completion-timeout)

(provide preview-completion-insert)
;;@doc
;; Whether to insert the completion suggestion on hover. Defaults to true.
(define preview-completion-insert helix.preview-completion-insert)

(provide completion-trigger-len)
;;@doc
;; Length to trigger completions
(define completion-trigger-len helix.completion-trigger-len)

(provide completion-replace)
;;@doc
;; Whether to instruct the LSP to replace the entire word when applying a
;; completion or to only insert new text
(define completion-replace helix.completion-replace)

(provide auto-info)
;;@doc
;; Whether to display infoboxes. Defaults to true.
(define auto-info helix.auto-info)

(provide true-color)
;;@doc
;; Set to `true` to override automatic detection of terminal truecolor support in the event of a
;; false negative. Defaults to `false`.
(define true-color helix.true-color)

(provide insert-final-newline)
;;@doc
;; Whether to automatically insert a trailing line-ending on write if missing. Defaults to `true`
(define insert-final-newline helix.insert-final-newline)

(provide color-modes)
;;@doc
;; Whether to color modes with different colors. Defaults to `false`.
(define color-modes helix.color-modes)

(provide gutters)
;;@doc
;; Gutter configuration
(define gutters helix.gutters)

(provide undercurl)
;;@doc
;; Set to `true` to override automatic detection of terminal undercurl support in the
;; event of a false negative. Defaults to `false`.
(define undercurl helix.undercurl)

(provide terminal)
;;@doc
;; Terminal config
(define terminal helix.terminal)

(provide rulers)
;;@doc
;; Column numbers at which to draw the rulers. Defaults to `[]`, meaning no rulers
(define rulers helix.rulers)

(provide bufferline)
;;@doc
;; Persistently display open buffers along the top
(define bufferline helix.bufferline)

(provide workspace-lsp-roots)
;;@doc
;; Workspace specific lsp ceiling dirs
(define workspace-lsp-roots helix.workspace-lsp-roots)

(provide default-line-ending)

;;@doc
;; Which line ending to choose for new documents.
;; Defaults to `native`. i.e. `crlf` on Windows, otherwise `lf`.
(define default-line-ending helix.default-line-ending)

(provide smart-tab)

;;@doc
;; Enables smart tab
(define smart-tab helix.smart-tab)

(provide rainbow-brackets)
;;@doc
;; Enables rainbow brackets
(define rainbow-brackets helix.rainbow-brackets)

(provide keybindings)
;;@doc Keybindings config
(define keybindings helix.keybindings)

(provide set-keybindings!)
;;@doc
;; Override the global keybindings with the provided keymap
(define set-keybindings! helix.set-keybindings!)

(provide inline-diagnostics-cursor-line-enable)
;;@doc
;; Inline diagnostics cursor line
(define inline-diagnostics-cursor-line-enable helix.inline-diagnostics-cursor-line-enable)

(provide inline-diagnostics-other-lines-disable)
;;@doc
;; Disable inline diagnostics for other lines
(define inline-diagnostics-other-lines-disable helix.inline-diagnostics-other-lines-disable)

(provide inline-diagnostics-cursor-line-disable)
;;@doc
;; Disable inline diagnostics for the cursor line
(define inline-diagnostics-cursor-line-disable helix.inline-diagnostics-cursor-line-disable)

(provide inline-diagnostics-end-of-line-disable)
;;@doc
;; Disable inline diagnostics for the end of the line
(define inline-diagnostics-end-of-line-disable helix.inline-diagnostics-end-of-line-disable)

(provide inline-diagnostics-other-lines-enable)
;;@doc
;; Inline diagnostics other lines
(define inline-diagnostics-other-lines-enable helix.inline-diagnostics-other-lines-enable)

(provide inline-diagnostics-end-of-line-enable)
;;@doc
;; Inline diagnostics end of line
(define inline-diagnostics-end-of-line-enable helix.inline-diagnostics-end-of-line-enable)

(provide inline-diagnostics-min-diagnostics-width)
;;@doc
;; Inline diagnostics min diagnostics width
(define inline-diagnostics-min-diagnostics-width helix.inline-diagnostics-min-diagnostics-width)

(provide inline-diagnostics-prefix-len)
;;@doc
;; Inline diagnostics prefix length
(define inline-diagnostics-prefix-len helix.inline-diagnostics-prefix-len)

(provide inline-diagnostics-max-wrap)
;;@doc
;; Inline diagnostics maximum wrap
(define inline-diagnostics-max-wrap helix.inline-diagnostics-max-wrap)

(provide inline-diagnostics-max-diagnostics)
;;@doc
;; Inline diagnostics max diagnostics
(define inline-diagnostics-max-diagnostics helix.inline-diagnostics-max-diagnostics)

(provide get-language-config)
;;@doc
;; Get the configuration for a specific language
(define get-language-config helix.get-language-config)

(provide set-language-config!)
;;@doc
;; Set the language configuration
(define set-language-config! helix.set-language-config!)

(provide ws-visible)
(define (ws-visible arg)
  (lambda (picker)
    (helix.ws-visible picker arg)
    picker))

(provide ws-chars)
(define (ws-chars arg)
  (lambda (picker)
    (helix.ws-chars picker arg)
    picker))

(provide ws-render)
(define (ws-render arg)
  (lambda (picker)
    (helix.ws-render picker arg)
    picker))

(provide ig-render)
(define (ig-render arg)
  (lambda (picker)
    (helix.ig-render picker arg)
    picker))

(provide ig-character)
(define (ig-character arg)
  (lambda (picker)
    (helix.ig-character picker arg)
    picker))

(provide ig-skip-levels)
(define (ig-skip-levels arg)
  (lambda (picker)
    (helix.ig-skip-levels picker arg)
    picker))

(provide sw-enable)
(define (sw-enable arg)
  (lambda (picker)
    (helix.sw-enable picker arg)
    picker))

(provide sw-max-wrap)
(define (sw-max-wrap arg)
  (lambda (picker)
    (helix.sw-max-wrap picker arg)
    picker))

(provide sw-max-indent-retain)
(define (sw-max-indent-retain arg)
  (lambda (picker)
    (helix.sw-max-indent-retain picker arg)
    picker))

(provide sw-wrap-indicator)
(define (sw-wrap-indicator arg)
  (lambda (picker)
    (helix.sw-wrap-indicator picker arg)
    picker))

(provide sw-wrap-at-text-width)
(define (sw-wrap-at-text-width arg)
  (lambda (picker)
    (helix.sw-wrap-at-text-width picker arg)
    picker))

(provide fp-hidden)
(define (fp-hidden arg)
  (lambda (picker)
    (helix.fp-hidden picker arg)
    picker))

(provide fp-follow-symlinks)
(define (fp-follow-symlinks arg)
  (lambda (picker)
    (helix.fp-follow-symlinks picker arg)
    picker))

(provide fp-deduplicate-links)
(define (fp-deduplicate-links arg)
  (lambda (picker)
    (helix.fp-deduplicate-links picker arg)
    picker))

(provide fp-parents)
(define (fp-parents arg)
  (lambda (picker)
    (helix.fp-parents picker arg)
    picker))

(provide fp-ignore)
(define (fp-ignore arg)
  (lambda (picker)
    (helix.fp-ignore picker arg)
    picker))

(provide fp-git-ignore)
(define (fp-git-ignore arg)
  (lambda (picker)
    (helix.fp-git-ignore picker arg)
    picker))

(provide fp-git-global)
(define (fp-git-global arg)
  (lambda (picker)
    (helix.fp-git-global picker arg)
    picker))

(provide fp-git-exclude)
(define (fp-git-exclude arg)
  (lambda (picker)
    (helix.fp-git-exclude picker arg)
    picker))

(provide fp-max-depth)
(define (fp-max-depth arg)
  (lambda (picker)
    (helix.fp-max-depth picker arg)
    picker))
