((comment) @injection.content
 (#set! injection.language "comment"))

; https://github.com/cross-rs/cross/blob/main/docs/config_file.md
(pair
  (bare_key) @_key (#eq? @_key "pre-build")
  (array
    (string) @injection.content)
  (#set! injection.language "bash"))
