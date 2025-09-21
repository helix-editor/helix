; (stmt_let) @assignment.outer

; (stmt_mut) @assignment.outer

; (stmt_const) @assignment.outer

; (stmt_let
;   value: (_) @assignment.inner)

; (stmt_mut
;   value: (_) @assignment.inner)

; (stmt_const
;   value: (_) @assignment.inner)

; (block) @block.outer

(comment) @comment.around

; (pipeline) @pipeline.outer

; (pipe_element) @pipeline.inner

(decl_def) @function.around

(decl_def
  body: (_) @function.inside)

; (ctrl_for) @loop.outer

; (ctrl_loop) @loop.outer

; (ctrl_while) @loop.outer

; (ctrl_for
;   body: (_) @loop.inner)

; (ctrl_loop
;   body: (_) @loop.inner)

; (ctrl_while
;   body: (_) @loop.inner)

; Conditional inner counts the last one, rather than the current one.
; (ctrl_if
;   then_branch: (_) @conditional.inner
;   else_block: (_)? @conditional.inner) @conditional.outer

(parameter) @parameter.around

; (command
;   head: (_) @call.inner) @call.outer

; (where_command
;   predicate: (_) @call.inner) @call.outer

; define pipeline first, because it should only match as a fallback
; e.g., `let a = date now` should match the whole assignment.
; But a standalone `date now` should also match a statement
; (pipeline) @statement.outer

; (stmt_let) @statement.outer

; (stmt_mut) @statement.outer

; (stmt_const) @statement.outer

; (ctrl_if) @statement.outer

; (ctrl_try) @statement.outer

; (ctrl_match) @statement.outer

; (ctrl_while) @statement.outer

; (ctrl_loop) @statement.outer

; (val_number) @number.inner
