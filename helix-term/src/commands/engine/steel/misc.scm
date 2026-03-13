(require-builtin helix/core/misc as helix.)
(provide hx.cx->pos)
;;@doc
;;DEPRECATED: Please use `cursor-position`
(define hx.cx->pos helix.hx.cx->pos)

(provide cursor-position)
;;@doc
;;Returns the cursor position within the current buffer as an integer
(define cursor-position helix.cursor-position)

(provide get-active-lsp-clients)
;;@doc
;;Get all language servers, that are attached to the current buffer
(define get-active-lsp-clients helix.get-active-lsp-clients)

(provide mode-switch-old)
;;@doc
;;Return the old mode from the event payload
(define mode-switch-old helix.mode-switch-old)

(provide mode-switch-new)
;;@doc
;;Return the new mode from the event payload
(define mode-switch-new helix.mode-switch-new)

(provide lsp-client-initialized?)
;;@doc
;;Return if the lsp client is initialized
(define lsp-client-initialized? helix.lsp-client-initialized?)

(provide lsp-client-name)
;;@doc
;;Get the name of the lsp client
(define lsp-client-name helix.lsp-client-name)

(provide lsp-client-offset-encoding)
;;@doc
;;Get the offset encoding of the lsp client
(define lsp-client-offset-encoding helix.lsp-client-offset-encoding)

(provide hx.custom-insert-newline)
;;@doc
;;DEPRECATED: Please use `insert-newline-hook`
(define hx.custom-insert-newline helix.hx.custom-insert-newline)

(provide insert-newline-hook)
;;@doc
;;Inserts a new line with the provided indentation.
;;
;;```scheme
;;(insert-newline-hook indent-string)
;;```
;;
;;indent-string : string?
;;
(define insert-newline-hook helix.insert-newline-hook)

(provide push-component!)
;;@doc
;;
;;Push a component on to the top of the stack.
;;
;;```scheme
;;(push-component! component)
;;```
;;
;;component : WrappedDynComponent?
;;
(define push-component! helix.push-component!)

(provide pop-last-component!)
;;@doc
;;DEPRECATED: Please use `pop-last-component-by-name!`
(define pop-last-component! helix.pop-last-component!)

(provide pop-last-component-by-name!)
;;@doc
;;Pops the last component off of the stack by name. In other words,
;;it removes the component matching this name from the stack.
;;
;;```scheme
;;(pop-last-component-by-name! name)
;;```
;;
;;name : string?
;;
(define pop-last-component-by-name! helix.pop-last-component-by-name!)

(provide on-key-callback)
;;@doc
;;
;;Enqueue a function to be run on the next keypress. The function must accept
;;a key event as an argument. This currently will only will work if the command is
;;called via a keybinding.
;;
(define on-key-callback helix.on-key-callback)

(provide trigger-on-key-callback)
;;@doc
;;
;;Trigger an on key callback if it exists with the specified key event.
;;
(define trigger-on-key-callback helix.trigger-on-key-callback)

(provide enqueue-thread-local-callback)
;;@doc
;;
;;Enqueue a function to be run following this context of execution. This could
;;be useful for yielding back to the editor in the event you want updates to happen
;;before your function is run.
;;
;;```scheme
;;(enqueue-thread-local-callback callback)
;;```
;;
;;callback : (-> any?)
;;    Function with no arguments.
;;
;;# Examples
;;
;;```scheme
;;(enqueue-thread-local-callback (lambda () (theme "focus_nova")))
;;```
;;
(define enqueue-thread-local-callback helix.enqueue-thread-local-callback)

(provide set-status!)
;;@doc
;;Sets the content of the status line, with the info severity
(define set-status! helix.set-status!)

(provide set-warning!)
;;@doc
;;Sets the content of the status line, with the warning severity
(define set-warning! helix.set-warning!)

(provide set-error!)
;;@doc
;;Sets the content of the status line, with the error severity
(define set-error! helix.set-error!)

(provide send-lsp-command)
;;@doc
;; Send an lsp command. The `lsp-name` must correspond to an active lsp.
;; The method name corresponds to the method name that you'd expect to see
;; with the lsp, and the params can be passed as a hash table. The callback
;; provided will be called with whatever result is returned from the LSP,
;; deserialized from json to a steel value.
;;
;; ```scheme
;; (send-lsp-command lsp-name method-name params callback)
;; ```
;;
;; # Example
;; ```scheme
;; (define (view-crate-graph)
;;   (send-lsp-command "rust-analyzer"
;;                     "rust-analyzer/viewCrateGraph"
;;                     (hash "full" #f)
;;                     ;; Callback to run with the result
;;                     (lambda (result) (displayln result))))
;; ```
(define send-lsp-command helix.send-lsp-command)

(provide send-lsp-notification)
;;@doc
;; Send an LSP notification. The `lsp-name` must correspond to an active LSP.
;; The method name corresponds to the method name that you'd expect to see
;; with the LSP, and the params can be passed as a hash table. Unlike
;; `send-lsp-command`, this does not expect a response and is used for
;; fire-and-forget notifications.
;;
;; ```scheme
;; (send-lsp-notification lsp-name method-name params)
;; ```
;;
;; # Example
;; ```scheme
;; (send-lsp-notification "copilot"
;;                        "textDocument/didShowCompletion"
;;                        (hash "item"
;;                              (hash "insertText" "a helpful suggestion"
;;                                    "range" (hash "start" (hash "line" 1 "character" 0)
;;                                                  "end" (hash "line" 1 "character" 2)))))
;; ```
(define send-lsp-notification helix.send-lsp-notification)

(provide lsp-reply-ok)
;;@doc
;; Send a successful reply to an LSP request with the given result.
;;
;; ```scheme
;; (lsp-reply-ok lsp-name request-id result)
;; ```
;;
;; * lsp-name : string? - Name of the language server
;; * request-id : string? - ID of the request to respond to
;; * result : any? - The result value to send back
;;
;; # Examples
;; ```scheme
;; ;; Reply to a request with id "123" from rust-analyzer
;; (lsp-reply-ok "rust-analyzer" "123" (hash "result" "value"))
;; ```
(define lsp-reply-ok helix.lsp-reply-ok)

(provide acquire-context-lock)
;;@doc
;;
;;Schedule a function to run on the main thread. This is a fairly low level function, and odds are
;;you'll want to use some abstractions on top of this.
;;
;;The provided function will get enqueued to run on the main thread, and during the duration of the functions
;;execution, the provided mutex will be locked.
;;
;;```scheme
;;(acquire-context-lock callback-fn mutex)
;;```
;;
;;callback-fn : (-> void?)
;;    Function with no arguments
;;
;;mutex : mutex?
(define acquire-context-lock helix.acquire-context-lock)

(provide enqueue-thread-local-callback-with-delay)
;;@doc
;;
;;Enqueue a function to be run following this context of execution, after a delay. This could
;;be useful for yielding back to the editor in the event you want updates to happen
;;before your function is run.
;;
;;```scheme
;;(enqueue-thread-local-callback-with-delay delay callback)
;;```
;;
;;delay : int?
;;    Time to delay the callback by in milliseconds
;;
;;callback : (-> any?)
;;    Function with no arguments.
;;
;;# Examples
;;
;;```scheme
;;(enqueue-thread-local-callback-with-delay 1000 (lambda () (theme "focus_nova"))) ;; Run after 1 second
;;``
;;
(define enqueue-thread-local-callback-with-delay helix.enqueue-thread-local-callback-with-delay)

(provide helix-await-callback)
;;@doc
;;DEPRECATED: Please use `await-callback`
(define helix-await-callback helix.helix-await-callback)

(provide await-callback)
;;@doc
;;
;;Await the given value, and call the callback function on once the future is completed.
;;
;;```scheme
;;(await-callback future callback)
;;```
;;
;;* future : future?
;;* callback (-> any?)
;;    Function with no arguments
(define await-callback helix.await-callback)

(provide add-inlay-hint)
;;@doc
;;
;;Warning: this is experimental
;;
;;Adds an inlay hint at the given character index. Returns the (first-line, last-line) list
;;associated with this snapshot of the inlay hints. Use this pair of line numbers to invalidate
;;the inlay hints.
;;
;;```scheme
;;(add-inlay-hint char-index completion) -> (list int? int?)
;;```
;;
;;char-index : int?
;;completion : string?
;;
(define add-inlay-hint helix.add-inlay-hint)

(provide remove-inlay-hint)
;;@doc
;;
;;Warning: this is experimental and should not be used.
;;This will most likely be removed soon.
;;
;;Removes an inlay hint at the given character index. Note - to remove
;;properly, the completion must match what was already there.
;;
;;```scheme
;;(remove-inlay-hint char-index completion)
;;```
;;
;;char-index : int?
;;completion : string?
;;
(define remove-inlay-hint helix.remove-inlay-hint)

(provide remove-inlay-hint-by-id)
;;@doc
;;
;;Warning: this is experimental
;;
;;Removes an inlay hint by the id that was associated with the added inlay hints.
;;
;;```scheme
;;(remove-inlay-hint first-line last-line)
;;```
;;
;;first-line : int?
;;last-line : int?
;;
(define remove-inlay-hint-by-id helix.remove-inlay-hint-by-id)
