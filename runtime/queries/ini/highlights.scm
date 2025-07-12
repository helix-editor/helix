(section_name
  (text) @type)

(comment) @comment

[
  "["
  "]"
] @punctuation.bracket

"=" @operator

(setting
  (setting_name) @variable.other.member
  ((setting_value) @string)?)
