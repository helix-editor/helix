(fenced_code_block
  (info_string) @injection.language
  (code_fence_content) @injection.content)

((html_block) @injection.content
 (#set! injection.language "html"))
((html_tag) @injection.content
 (#set! injection.language "html"))
