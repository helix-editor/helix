
(atx_heading) @text.title

[
  (code_span)
  (fenced_code_block)
]@text.literal

(code_block
"```" @escape
(_)
"```" @escape) 

[
  (link_text)
  (image_description)
] @text.strong

[
  (emphasis)
  (strong_emphasis)
] @text.emphasis
(link_destination) @text.uri

(html_comment) @comment