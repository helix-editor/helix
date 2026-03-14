((setting
  key: (key) @key
  (#match? @key "^script$|-script$|^script-")
  value: (_
    (string_content) @injection.content))
  (#set! "injection.language" "bash"))

; Requires no spacing around "=" in environment settings for proper highlighting.
; Could be improved if Tree-sitter allowed to specify the target node of the injected
; language, instead of always using the root node.
; See this proposal:
; https://github.com/tree-sitter/tree-sitter/issues/3625
((task_section
  (sub_section_2
    name: (_) @section_name
    (#eq? @section_name "environment")
    (setting) @injection.content))
  (#set! "injection.language" "bash")
  (#set! injection.combined)
  (#set! injection.include-children))
