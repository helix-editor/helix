; Upstream: https://github.com/alex-pinkus/tree-sitter-swift/blob/57c1c6d6ffa1c44b330182d41717e6fe37430704/queries/injections.scm

; Parse regex syntax within regex literals

((regex_literal) @injection.content
 (#set! injection.language "regex"))

((comment) @injection.content
 (#set! injection.language "comment")
 (#set! injection.include-children))
