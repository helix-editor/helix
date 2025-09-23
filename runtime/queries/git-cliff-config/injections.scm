((comment) @injection.content
 (#set! injection.language "comment"))

; https://git-cliff.org/docs/configuration/changelog
(table
  (bare_key) @_table (#eq? @_table "changelog")
  (pair
    (bare_key) @_key (#any-of? @_key "header" "body" "footer")
    (string) @injection.content
  (#set! injection.language "tera")))

; https://git-cliff.org/docs/configuration/git#commit_preprocessors
; https://git-cliff.org/docs/configuration/git/#link_parsers
; https://git-cliff.org/docs/configuration/changelog#postprocessors
; https://git-cliff.org/docs/configuration/git/#tag_pattern
; https://git-cliff.org/docs/configuration/git/#skip_tags
; https://git-cliff.org/docs/configuration/git/#ignore_tags
; https://git-cliff.org/docs/configuration/git/#count_tags
; https://git-cliff.org/docs/configuration/bump/#custom_major_increment_regex--custom_minor_increment_regex
(pair
  (bare_key) @_key (#any-of? @_key
    "pattern"
    "tag_pattern"
    "skip_tags"
    "ignore_tags"
    "count_tags"
    "custom_major_increment_regex"
    "custom_minor_increment_regex"
  )
  (string) @injection.content
  (#set! injection.language "regex"))

; https://git-cliff.org/docs/configuration/git/#commit_preprocessors
; [[git.commit_preprocessors]]
; replace_command = ""
(pair
  (bare_key) @_key (#eq? @_key "replace_command")
  (string) @injection.content
  (#set! injection.language "bash"))

; https://git-cliff.org/docs/configuration/git/#commit_parsers
; [[git.commit_parsers]]
; message = "..."
(table
  (bare_key) @_table (#eq? @_table "git")
  (pair
    (bare_key) @_key (#eq? @_key "commit_parsers")
    (array
      (inline_table
        (pair
          (bare_key) @_message (#any-of? @_message "message" "body")
          (string) @injection.content))))
  (#set! injection.language "regex"))
