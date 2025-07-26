(comment) @comment
[
  (environment_variable)
  (placeholder)
] @constant

[
  (network_address)
  (ip_address_or_cidr)
] @string.special.url

(path) @string.special.path

[
  (snippet_name)
  (named_route_identifier)
  (site_address)
] @keyword

(directive (directive_name) @variable.other.member)

; declaration of a named matcher
(named_matcher (matcher_identifier (matcher_name)) @function.macro)

; reference to a named matcher
(matcher (matcher_identifier (matcher_name)) @function.macro)

; directive within a named matcher declaration
(matcher_directive (matcher_directive_name) @function.method)

; any other matcher (wildcard and path)
(matcher) @function.macro

[
  (interpreted_string_literal)
  (raw_string_literal)
  (heredoc)
  (cel_expression)
] @string
(escape_sequence) @constant.character.escape

[
  (duration_literal)
  (int_literal)
] @constant.numeric

[
  "{"
  "}"
] @punctuation.bracket

(global_options
  (directive) @keyword.directive)

(directive
  name: (directive_name)
  (argument) @type)

; matches directive arguments that looks like an absolute path
; e.g.
; log {
;     output file /var/log/caddy.log
; }
(directive
  (argument) @string.special.path
  (#match? @string.special.path "^/"))

((argument) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "on" "off"))

((argument) @type.enum.variant
  (#any-of? @type.enum.variant "tcp" "udp" "ipv4" "ipv6"))
