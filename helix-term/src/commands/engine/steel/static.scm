;; Note: Most of the remaining static commands are templated and generated
;; from the runtime.

(require-builtin helix/core/static as helix.static.)

(provide insert_char)
;;@doc
;;Insert a given character at the cursor cursor position
(define insert_char helix.static.insert_char)

(provide insert_string)
;;@doc
;;Insert a given string at the current cursor position
(define insert_string helix.static.insert_string)

(provide set-current-selection-object!)
;;@doc
;;Update the selection object to the current selection within the editor
(define set-current-selection-object! helix.static.set-current-selection-object!)

(provide push-range-to-selection!)
;;@doc
;;Push a new range to a selection. The new selection will be the primary one
(define push-range-to-selection! helix.static.push-range-to-selection!)

(provide set-current-selection-primary-index!)
;;@doc
;;Set the primary index of the current selection
(define set-current-selection-primary-index! helix.static.set-current-selection-primary-index!)

(provide remove-current-selection-range!)
;;@doc
;;Remove a range from the current selection
(define remove-current-selection-range! helix.static.remove-current-selection-range!)

(provide regex-selection)
;;@doc
;;Run the given regex within the existing buffer
(define regex-selection helix.static.regex-selection)

(provide replace-selection-with)
;;@doc
;;Replace the existing selection with the given string
(define replace-selection-with helix.static.replace-selection-with)

(provide enqueue-expression-in-engine)
;;@doc
;;Enqueue an expression to run at the top level context,
;;        after the existing function context has exited.
(define enqueue-expression-in-engine helix.static.enqueue-expression-in-engine)

(provide get-current-line-character)
;;@doc
;;Returns the current column number with the given position encoding
(define get-current-line-character helix.static.get-current-line-character)

(provide cx->current-file)
;;@doc
;;Get the currently focused file path
(define cx->current-file helix.static.cx->current-file)

(provide current_selection)
;;@doc
;;Returns the current selection as a string
(define current_selection helix.static.current_selection)

(provide current-selection->string)
;;@doc
;;Returns the current selection as a string
(define current-selection->string helix.static.current-selection->string)

(provide load-buffer!)
;;@doc
;;Evaluates the current buffer
(define load-buffer! helix.static.load-buffer!)

(provide current-highlighted-text!)
;;@doc
;;Returns the currently highlighted text as a string
(define current-highlighted-text! helix.static.current-highlighted-text!)

(provide get-current-line-number)
;;@doc
;;Returns the current line number
(define get-current-line-number helix.static.get-current-line-number)

(provide get-current-column-number)
;;@doc
;;Returns the visual current column number of unicode graphemes
(define get-current-column-number helix.static.get-current-column-number)

(provide current-selection-object)
;;@doc
;;Returns the current selection object
(define current-selection-object helix.static.current-selection-object)

(provide get-helix-cwd)
;;@doc
;;Returns the current working directly that helix is using
(define get-helix-cwd helix.static.get-helix-cwd)

(provide move-window-far-left)
;;@doc
;;Moves the current window to the far left
(define move-window-far-left helix.static.move-window-far-left)

(provide move-window-far-right)
;;@doc
;;Moves the current window to the far right
(define move-window-far-right helix.static.move-window-far-right)

(provide selection->primary-index)
;;@doc
;;Returns index of the primary selection
(define selection->primary-index helix.static.selection->primary-index)

(provide selection->primary-range)
;;@doc
;;Returns the range for primary selection
(define selection->primary-range helix.static.selection->primary-range)

(provide selection->ranges)
;;@doc
;;Returns all ranges of the selection
(define selection->ranges helix.static.selection->ranges)

(provide range-anchor)
;;@doc
;;Get the anchor of the range: the side that doesn't move when extending.
(define range-anchor helix.static.range-anchor)

(provide range->from)
;;@doc
;;Get the start of the range
(define range->from helix.static.range->from)

(provide range-head)
;;@doc
;;Get the head of the range, moved when extending.
(define range-head helix.static.range-head)

(provide range->to)
;;@doc
;;Get the end of the range
(define range->to helix.static.range->to)

(provide range->span)
;;@doc
;;Get the span of the range (from, to)
(define range->span helix.static.range->span)

(provide range)
;;@doc
;;Construct a new range object
;;
;;```scheme
;;(range anchor head) -> Range?
;;```
;;
(define range helix.static.range)

(provide range->selection)
;;@doc
;;Convert a range into a selection
(define range->selection helix.static.range->selection)

(provide get-helix-scm-path)
;;@doc
;;Returns the path to the helix.scm file as a string
(define get-helix-scm-path helix.static.get-helix-scm-path)

(provide get-init-scm-path)
;;@doc
;;Returns the path to the init.scm file as a string
(define get-init-scm-path helix.static.get-init-scm-path)
