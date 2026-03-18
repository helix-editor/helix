; inherits: html

(raw_text) @none

((tag_name) @type
  (#match? @type "^[A-Z]"))

(tag_name
  namespace: (tag_namespace) @keyword
  ":" @punctuation.delimiter
  name: (tag_local_name) @tag)

(tag_name
  object: (tag_member) @type
  "." @punctuation.delimiter
  property: (tag_member) @tag)

(attribute_directive) @keyword
(attribute_name ":" @punctuation.delimiter)
(attribute_identifier) @property
(attribute_modifier) @attribute
(attribute_modifiers "|" @punctuation.delimiter)

(expression) @embedded

[
  "as"
  "key"
  "html"
  "debug"
  "snippet"
  "render"
  "attach"
] @keyword

"const" @keyword.storage.modifier

[
  "if"
  "else if"
  "else"
  "then"
  "await"
] @keyword.control.conditional

"each" @keyword.control.repeat

"catch" @keyword.control.exception

(block_keyword) @keyword
(branch_kind) @keyword.control.conditional
(shorthand_kind) @keyword.control.conditional

[
  (block_open)
  (block_close)
] @punctuation.bracket

(snippet_name) @function
(snippet_type_parameters) @type

(snippet_parameters
  parameter: (pattern) @variable)

(each_block binding: (pattern) @variable)
(each_block index: (pattern) @variable)
(await_branch (pattern) @variable)
(await_block (pattern) @variable)
(orphan_branch (pattern) @variable)

(tag_comment kind: (line_comment) @comment)
(tag_comment kind: (block_comment) @comment)

[
  "("
  ")"
  ","
  "|"
] @punctuation.delimiter
