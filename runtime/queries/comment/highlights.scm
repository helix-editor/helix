[
 "("
 ")"
] @punctuation.bracket

":" @punctuation.delimiter

; Hint level tags
((tag (name) @hint)
 (#match? @hint "^(HINT|MARK)$"))

("text" @hint
 (#match? @hint "^(HINT|MARK)$"))

; Info level tags
((tag (name) @info)
 (#match? @info "^(INFO|NOTE|TODO)$"))

("text" @info
 (#match? @info "^(INFO|NOTE|TODO)$"))

; Warning level tags
((tag (name) @warning)
 (#match? @warning "^(HACK|WARN|WARNING)$"))

("text" @warning
 (#match? @warning "^(HACK|WARN|WARNING)$"))

; Error level tags
((tag (name) @error)
 (match? @error "^(BUG|FIXME|ISSUE|XXX)$"))

("text" @error
 (match? @error "^(BUG|FIXME|ISSUE|XXX)$"))

(tag
 (name) @ui.text
 (user)? @constant)

; Issue number (#123)
("text" @constant.numeric
 (#match? @constant.numeric "^#[0-9]+$"))

; User mention (@user)
("text" @tag
 (#match? @tag "^[@][a-zA-Z0-9_-]+$"))
