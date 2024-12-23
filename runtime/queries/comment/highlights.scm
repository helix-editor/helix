; Hint level tags
((tag (name) @hint)
 (#match? @hint "^(HINT|MARK|PASSED|STUB|MOCK)$"))

("text" @hint
 (#match? @hint "^(HINT|MARK|PASSED|STUB|MOCK)$"))

; Info level tags
((tag (name) @info)
 (#match? @info "^(INFO|NOTE|TODO|PERF|OPTIMIZE|PERFORMANCE|QUESTION|ASK)$"))

("text" @info
 (#match? @info "^(INFO|NOTE|TODO|PERF|OPTIMIZE|PERFORMANCE|QUESTION|ASK)$"))

; Warning level tags
((tag (name) @warning)
 (#match? @warning "^(HACK|WARN|WARNING|TEST|TEMP)$"))

("text" @warning
 (#match? @warning "^(HACK|WARN|WARNING|TEST|TEMP)$"))

; Error level tags
((tag (name) @error)
 (#match? @error "^(BUG|FIXME|ISSUE|XXX|FIX|SAFETY|FIXIT|FAILED|DEBUG|INVARIANT|COMPLIANCE)$"))

("text" @error
 (#match? @error "^(BUG|FIXME|ISSUE|XXX|FIX|SAFETY|FIXIT|FAILED|DEBUG|INVARIANT|COMPLIANCE)$"))

(tag
 (name) @ui.text
 (user)? @constant)

; Issue number (#123)
("text" @constant.numeric
 (#match? @constant.numeric "^#[0-9]+$"))

; User mention (@user)
("text" @tag
 (#match? @tag "^[@][a-zA-Z0-9_-]+$"))

(uri) @markup.link.url
