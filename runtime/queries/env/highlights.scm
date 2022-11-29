(env_variable (quoted_string)) @string
(env_variable (unquoted_string)) @string

(env_key) @keyword

((variable) @keyword
  (#match? @keyword "^([A-Z][A-Z_0-9]*)$"))
  
[
  "{"
  "}"
] @punctuation.bracket

[
  "$"
  "=" 
] @operator

(comment) @comment