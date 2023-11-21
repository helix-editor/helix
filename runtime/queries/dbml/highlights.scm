
; comments highlighting
(comment) @comment

; keyword highlighting
(keyword_def) @keyword
(keyword_enum) @keyword
(keyword_ref) @keyword

; identify blocks and definitions
(block) @class
(definition) @function

; for identifiers
(identifier) @variable
(type) @keyword

; Highlight special types for database/data types
("Project" ) @type
("Table" ) @type
("TableGroup" ) @type
("database_type" ) @variable

; Index related highlighting
(index_block) @class

; string and number constants
("'''") @string.escape
(string) @string
(number) @constant.numeric

; brackets
[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

; brackets
[
  ":"
  "."
  ","
] @punctuation.delimeter

