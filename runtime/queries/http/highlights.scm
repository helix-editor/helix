; Comments
(comment) @comment

(method) @keyword

(target_url) @string.special.url
(host) @string.special.url
(path) @string.special.path
(scheme) @keyword

(http_version) @keyword

(authority
  (pair
    name: (_) @info
    value: (_) @warning))

(pair
  name: (_) @attribute
  value: (_) @string)

(query_param 
  key: (key) @attribute
  value: (value) @string)

(header
  name: (name) @constant
  value: (value) @string)

(external_body
  file_path: (path) @string.special.path) @keyword

(number) @constant.numeric
(string) @string
(variable) @variable.other.member
(variable_declaration 
  identifier: (identifier) @variable)

[
  "@"
  ":"
  "="
] @operator
