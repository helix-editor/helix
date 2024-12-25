((section_name) @keyword.directive
 (#eq? @keyword.directive "include"))

((section_header
   (section_name) @keyword.directive
   (subsection_name))
 (#eq? @keyword.directive "includeIf"))

(section_name) @markup.heading
(variable (name) @variable.other.member)
[(true) (false)] @constant.builtin.boolean
(integer) @constant.numeric.integer

((string) @string.special.path
 (#match? @string.special.path "^(~|./|/)"))

[(string) (subsection_name)] @string

[
  "["
  "]"
] @punctuation.bracket

["=" "\\"] @punctuation.delimiter

(escape_sequence) @constant.character.escape

(comment) @comment
