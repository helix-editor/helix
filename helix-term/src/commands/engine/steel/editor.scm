(require-builtin helix/core/editor as helix.)

(provide register-hook)
;;@doc
;; Register a hook to be called after the event kind fired. It is not possible
;; to unregister a hook once it has been registered. Any values that are captured
;; through the callback function for this hook are considered to be rooted,
;; and will not be freed for the duration of the runtime.
;;
;; ```scheme
;; (register-hook event-kind callback-fn)
;;
;; event-kind - symbol?
;; callback-fn - function?
;; ```
;;
;; The valid events are as follows:
;; * 'on-mode-switch
;; * 'post-insert-char
;; * 'post-command
;; * 'document-focus-lost
;; * 'selection-did-change
;; * 'document-opened
;; * 'document-saved
;; * 'document-changed
;; * 'document-closed
;;
;; Each of these expects a function with a slightly different signature to accept
;; the event payload.
;;
;; ## on-mode-switch
;;
;; Expects a function with one argument to accept the `OnModeSwitchEvent`.
;;
;; ### Example:
;; ```scheme
;; (register-hook 'on-mode-switch (lambda (switch-event) (log::info! (mode-switch-old switch-event))))
;; ```
;;
;; ## post-insert-char
;;
;; Expects a function with one argument to accept the character (`char?`).
;;
;; ```scheme
;; (register-hook 'post-insert-char
;;          (lambda (char) (log::info! char)))
;; ```
;;
;; ## post-command
;;
;; Post command expects a function with one argument to accept the name of the command that was called.
;; Note, this does not provide the arguments for the command, just the name of the command.
;;
;; ```scheme
;; (register-hook 'post-command
;;                (lambda (command-name) (log::info! command-name)))
;; ```
;;
;; ## document-focus-lost
;;
;; Expects a function with one argument to accept the doc id of the document that has lost focus.
;;
;; ## selection-did-change
;;
;; Expects a function with one argument to accept the view id.
;;
;; ## document-opened
;;
;; Expects a function with one argument to accept the doc id of the document that was just opened.
;;
;; ## document-saved
;;
;; Expects a function with one argument to accept the doc id of the document that was just saved.
;; ## document-changed
;;
;; Expects a function with two arguments to accept the doc id of the docuoment that was just saved and the old text of the document before the change.
;; ## document-closed
;;
;; Expects a function with one argument to accept the `OnDocClosedEvent`
;; ### Example:
;; ```scheme
;; (register-hook 'document-closed (lambda (closed-event) (log::info! (doc-closed-id closed-event))))
(define (register-hook event-kind callback-fn)
  (helix.register-hook event-kind callback-fn))

;;@doc
;; Get the ID of the closed document
(provide doc-closed-id)
(define doc-closed-id (helix.doc-closed-id))

;;@doc
;; Get the language of the closed document
(provide doc-closed-language)
(define doc-closed-language (helix.doc-closed-language))

;;@doc
;; Get the text of the closed document
(provide doc-closed-text)
(define doc-closed-text (helix.doc-closed-text))

;;@doc
;; Get the path of the closed document
(provide doc-closed-path)
(define doc-closed-path (helix.doc-closed-path))

(provide editor-focus)
;;@doc
;;
;;Get the current focus of the editor, as a `ViewId`.
;;
;;```scheme
;;(editor-focus) -> ViewId
;;```
;;
(define editor-focus helix.editor-focus)

(provide editor-mode)
;;@doc
;;
;;Get the current mode of the editor
;;
;;```scheme
;;(editor-mode) -> Mode?
;;```
;;
(define editor-mode helix.editor-mode)

(provide cx->themes)
;;@doc
;;DEPRECATED: Please use `themes->list`
(define cx->themes helix.cx->themes)

(provide editor-count)
;;@doc
;;Get the count
(define editor-count helix.editor-count)

(provide themes->list)
;;@doc
;;
;;Get the current themes as a list of strings.
;;
;;```scheme
;;(themes->list) -> (listof string?)
;;```
;;
(define themes->list helix.themes->list)

(provide editor-all-documents)
;;@doc
;;
;;Get a list of all of the document ids that are currently open.
;;
;;```scheme
;;(editor-all-documents) -> (listof DocumentId?)
;;```
;;
(define editor-all-documents helix.editor-all-documents)

(provide cx->cursor)
;;@doc
;;DEPRECATED: Please use `current-cursor`
(define cx->cursor helix.cx->cursor)

(provide current-cursor)
;;@doc
;;Gets the primary cursor position in screen coordinates,
;;or `#false` if the primary cursor is not visible on screen.
;;
;;```scheme
;;(current-cursor) -> (listof? (or Position? #false) CursorKind)
;;```
;;
(define current-cursor helix.current-cursor)

(provide editor-focused-buffer-area)
;;@doc
;;
;;Get the `Rect` associated with the currently focused buffer.
;;
;;```scheme
;;(editor-focused-buffer-area) -> (or Rect? #false)
;;```
;;
(define editor-focused-buffer-area helix.editor-focused-buffer-area)

(provide selected-register!)
;;@doc
;;Get currently selected register
(define selected-register! helix.selected-register!)

(provide Action/Load)
(define Action/Load helix.Action/Load)

(provide Action/Replace)
(define Action/Replace helix.Action/Replace)

(provide Action/HorizontalSplit)
(define Action/HorizontalSplit helix.Action/HorizontalSplit)

(provide Action/VerticalSplit)
(define Action/VerticalSplit helix.Action/VerticalSplit)

(provide set-editor-count!)
;;@doc
;;Sets the editor count.
(define (set-editor-count! arg)
  (helix.set-editor-count! *helix.cx* arg))

(provide string->editor-mode)
;;@doc
;;
;;Create an editor mode from a string, or false if it string was not one of
;;"normal", "insert", or "select"
;;
;;```scheme
;;(string->editor-mode "normal") -> (or Mode? #f)
;;```
;;
(define string->editor-mode helix.string->editor-mode)

(provide editor->doc-id)
;;@doc
;;Get the document from a given view.
(define editor->doc-id helix.editor->doc-id)

(provide editor-switch!)
;;@doc
;;Open the document in a vertical split.
(define editor-switch! helix.editor-switch!)

(provide editor-set-focus!)
;;@doc
;;Set focus on the view.
(define editor-set-focus! helix.editor-set-focus!)

(provide editor-set-mode!)
;;@doc
;;Set the editor mode.
(define editor-set-mode! helix.editor-set-mode!)

(provide editor-doc-in-view?)
;;@doc
;;Check whether the current view contains a document.
(define editor-doc-in-view? helix.editor-doc-in-view?)

(provide set-scratch-buffer-name!)
;;@doc
;;Set the name of a scratch buffer.
(define set-scratch-buffer-name! helix.set-scratch-buffer-name!)

(provide set-buffer-uri!)
;;@doc
;;Set the URI of the buffer
(define set-buffer-uri! helix.set-buffer-uri!)

(provide editor-doc-exists?)
;;@doc
;;Check if a document exists.
(define editor-doc-exists? helix.editor-doc-exists?)

(provide editor-document-last-saved)
;;@doc
;;Check when a document was last saved (returns a `SystemTime`)
(define editor-document-last-saved helix.editor-document-last-saved)

(provide editor-document->language)
;;@doc
;;Get the language for the document
(define editor-document->language helix.editor-document->language)

(provide editor-document-dirty?)
;;@doc
;;Check if a document has unsaved changes
(define editor-document-dirty? helix.editor-document-dirty?)

(provide editor-document-reload)
;;@doc
;;Reload a document.
(define editor-document-reload helix.editor-document-reload)

(provide editor->text)
;;@doc
;;Get the document as a rope.
(define editor->text helix.editor->text)

(provide editor-document->path)
;;@doc
;;Get the path to a document.
(define editor-document->path helix.editor-document->path)

(provide register->value)
;;@doc
;;Get register value as a list of strings.
(define register->value helix.register->value)

(provide set-editor-clip-top!)
;;@doc
;;Set the editor clipping at the top.
(define set-editor-clip-top! helix.set-editor-clip-top!)

(provide set-editor-clip-right!)
;;@doc
;;Set the editor clipping at the right.
(define set-editor-clip-right! helix.set-editor-clip-right!)

(provide set-editor-clip-left!)
;;@doc
;;Set the editor clipping at the left.
(define set-editor-clip-left! helix.set-editor-clip-left!)

(provide set-editor-clip-bottom!)
;;@doc
;;Set the editor clipping at the bottom.
(define set-editor-clip-bottom! helix.set-editor-clip-bottom!)

(provide editor-switch-action!)
(define editor-switch-action! helix.editor-switch-action!)

(provide set-register!)
(define set-register! helix.set-register!)
