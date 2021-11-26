[
 "("
 ")"
] @punctuation.bracket

":" @punctuation.delimiter

(tag (name) @ui.text (user)? @constant)

((tag ((name) @warning))
 (#any-of? @warning "TODO" "HACK" "WARNING"))

("text" @warning
 (#any-of? @warning "TODO" "HACK" "WARNING"))

((tag ((name) @error))
 (#any-of? @error "FIXME" "XXX" "BUG"))

("text" @error
 (#any-of? @error "FIXME" "XXX" "BUG"))

; Issue number (#123)
; ("text" @number (#lua-match? @number "^#[0-9]+$"))
; User mention (@user)
; ("text" @constant-numeric (#lua-match? @constant-numeric "^[@][a-zA-Z0-9_-]+$"))