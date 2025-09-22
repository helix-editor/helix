;; Harneet Programming Language - Language Injection Rules for Helix Editor
;; Tree-sitter injection queries for embedded languages

;; SQL strings (common pattern in many languages)
((string_literal) @injection.content
 (#match? @injection.content "(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)")
 (#set! injection.language "sql"))

;; JSON strings  
((string_literal) @injection.content
 (#match? @injection.content "^[\s]*[{\[]")
 (#set! injection.language "json"))

;; HTML strings
((string_literal) @injection.content
 (#match? @injection.content "^[\s]*<[^>]+>")
 (#set! injection.language "html"))

;; Regular expressions (if Harneet adds regex support)
((string_literal) @injection.content
 (#match? @injection.content "^/.*/$")
 (#set! injection.language "regex"))

;; Comments with embedded code examples
((line_comment) @injection.content
 (#match? @injection.content "// Example:")
 (#set! injection.language "harneet"))

((block_comment) @injection.content
 (#match? @injection.content "/\\*.*Example:")
 (#set! injection.language "harneet"))