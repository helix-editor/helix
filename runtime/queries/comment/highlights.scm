(tag
 (name) @ui.text
 (user)? @constant)

; Hint level tags
((tag (name) @hint)
 (#any-of? @hint "HINT" "MARK" "PASSED" "STUB" "MOCK"))

("text" @hint
 (#any-of? @hint "HINT" "MARK" "PASSED" "STUB" "MOCK"))

; Info level tags
((tag (name) @info)
 (#any-of? @info "INFO" "NOTE" "TODO" "PERF" "OPTIMIZE" "PERFORMANCE" "QUESTION" "ASK"))

("text" @info
 (#any-of? @info "INFO" "NOTE" "TODO" "PERF" "OPTIMIZE" "PERFORMANCE" "QUESTION" "ASK"))

; Warning level tags
((tag (name) @warning)
 (#any-of? @warning "HACK" "WARN" "WARNING" "TEST" "TEMP"))

("text" @warning
 (#any-of? @warning "HACK" "WARN" "WARNING" "TEST" "TEMP"))

; Error level tags
((tag (name) @error)
 (#any-of? @error "BUG" "FIXME" "ISSUE" "XXX" "FIX" "SAFETY" "FIXIT" "FAILED" "DEBUG" "INVARIANT" "COMPLIANCE"))

("text" @error
 (#any-of? @error "BUG" "FIXME" "ISSUE" "XXX" "FIX" "SAFETY" "FIXIT" "FAILED" "DEBUG" "INVARIANT" "COMPLIANCE"))

; Issue number (#123)
("text" @constant.numeric
 (#match? @constant.numeric "^#[0-9]+$"))

; User mention (@user)
("text" @tag
 (#match? @tag "^[@][a-zA-Z0-9_-]+$"))

(uri) @markup.link.url
