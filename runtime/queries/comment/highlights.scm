(tag
 (name) @ui.text
 (user)? @constant)

; Hint level tags
((tag (name) @hint)
 (#any-of? @hint "HINT" "MARK" "PASSED" "STUB" "MOCK" "TIP" "IDEA" "EVIDENCE-OF" "COM" "CHECK" "CHECK-NEXT" "CHECK-SAME" "CHECK-EMPTY" "CHECK-NOT" "CHECK-COUNT" "CHECK-DAG" "CHECK-LABEL"))

("text" @hint
 (#any-of? @hint "HINT" "MARK" "PASSED" "STUB" "MOCK" "TIP" "IDEA" "EVIDENCE-OF" "COM" "CHECK" "CHECK-NEXT" "CHECK-SAME" "CHECK-EMPTY" "CHECK-NOT" "CHECK-COUNT" "CHECK-DAG" "CHECK-LABEL"))

; Info level tags
((tag (name) @info)
 (#any-of? @info "INFO" "NOTE" "TODO" "TO-DO" "PERF" "OPTIMIZE" "PERFORMANCE" "QUESTION" "ASK" "REVIEW" "PR" "CR" "MAGIC" "WHY" "REASON" "NB" "N.B."))

("text" @info
 (#any-of? @info "INFO" "NOTE" "TODO" "TO-DO" "PERF" "OPTIMIZE" "PERFORMANCE" "QUESTION" "ASK" "REVIEW" "PR" "CR" "MAGIC" "WHY" "REASON" "NB" "N.B."))

; Warning level tags
((tag (name) @warning)
 (#any-of? @warning "HACK" "WARN" "WARNING" "TEST" "TESTING" "TEMP" "CAREFUL" "CAUTION" "TRICKY" "FRAGILE"))

("text" @warning
 (#any-of? @warning "HACK" "WARN" "WARNING" "TEST" "TESTING" "TEMP" "CAREFUL" "CAUTION" "TRICKY" "FRAGILE"))

; Error level tags
((tag (name) @error)
 (#any-of? @error "BUG" "FIXME" "ISSUE" "XXX" "FIX" "SAFETY" "FIXIT" "FAILED" "DEBUG" "INVARIANT" "COMPLIANCE" "PANIC" "SECURITY" "ASSUMPTION" "ASSUME" "POSTCONDITION" "PRECONDITION" "PROOF"))

("text" @error
 (#any-of? @error "BUG" "FIXME" "ISSUE" "XXX" "FIX" "SAFETY" "FIXIT" "FAILED" "DEBUG" "INVARIANT" "COMPLIANCE" "PANIC" "SECURITY" "ASSUMPTION" "ASSUME" "POSTCONDITION" "PRECONDITION" "PROOF"))

; Issue number (#123)
("text" @constant.numeric
 (#match? @constant.numeric "^#[0-9]+$"))

; User mention (@user)
("text" @tag
 (#match? @tag "^[@][a-zA-Z0-9_-]+$"))

(uri) @markup.link.url
