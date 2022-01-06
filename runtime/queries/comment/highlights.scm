[
 "("
 ")"
] @punctuation.bracket

":" @punctuation.delimiter

((tag (name) @warning)
 (#match? @warning "^(TODO|HACK|WARNING)$"))

("text" @warning
 (#match? @warning "^(TODO|HACK|WARNING)$"))

((tag (name) @error)
 (match? @error "^(FIXME|XXX|BUG)$"))

("text" @error
 (match? @error "^(FIXME|XXX|BUG)$"))

(tag
 (name) @ui.text
 (user)? @constant)

; Issue number (#123)
("text" @constant.numeric
 (#match? @constant.numeric "^#[0-9]+$"))

; User mention (@user)
("text" @tag
 (#match? @tag "^[@][a-zA-Z0-9_-]+$"))
