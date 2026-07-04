; block commands
(block_command
  command: _ @keyword)
(block_command
  ["@" "\\"] @keyword )
(block_command
  (parameter) @variable.parameter)
(block_command
  (description (content) @string))

; inline commands
(inline_command
  command: _ @keyword)
(inline_command
  ["@" "\\"] @keyword )
(inline_command
  (parameter) @variable.parameter)

; @param command
(param_attribute) @keyword.storage

; markdown
(header) @markup.heading
(link) @markup.link.text
(link
  (link_uri) @markup.link.url)
(link
  (link_ref) @markup.link.url)
(email) @markup.link.url

; styling
(bold) @markup.bold
(inline_command command: "b"
  (_) @markup.bold)
(italic) @markup.italic
(inline_command command: ["e" "em" "a"]
  (_) @markup.italic)
(code) @markup.raw.inline
(inline_command command: "c"
  (_) @markup.raw.inline)
(code_block
  (code_line) @markup.raw.block)

; let important commands stick out
(block_command
  command: _ @_cmd @keyword
  (description (content) @warning)
  (#any-of? @_cmd "warning" "note"))

(block_command
  command: _ @_cmd @keyword
  (description (content) @markup.heading)
  (#any-of? @_cmd "brief"))
