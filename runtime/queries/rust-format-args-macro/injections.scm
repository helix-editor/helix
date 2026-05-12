; inherits: rust

; HACK: This language is the same as Rust but all strings are injected
; with rust-format-args. Rust injects this into known macros which use
; the format args syntax. This can cause false-positive highlights but
; those are expected to be rare.

([
   (string_literal (string_content) @injection.content)
   (raw_string_literal (string_content) @injection.content)
 ]
 (#set! injection.language "rust-format-args")
 (#set! injection.include-children))
