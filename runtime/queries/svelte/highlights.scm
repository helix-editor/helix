; Tag names
(tag_name) @tag

; Erroneous/mismatched end tags
(erroneous_end_tag_name) @error

; DOCTYPE declaration
(doctype) @constant

; Attribute names
(attribute_name) @attribute

; Attribute values
(attribute_value) @string
(quoted_attribute_value) @string

; Comments
(comment) @comment

; Character entities
(entity) @string.special.symbol

; Text content
(text) @text

; Raw text in script/style
(raw_text) @none

; HTML punctuation
[
  "<"
  ">"
  "</"
  "/>"
  "<!"
] @punctuation.bracket

"=" @punctuation.delimiter

; Component tags (PascalCase)
((tag_name) @type (#match? @type "^[A-Z]"))

; Namespaced tags (svelte:component, svelte:self, etc.)
(tag_name
  namespace: (tag_namespace) @keyword
  ":" @punctuation.delimiter
  name: (tag_local_name) @tag)

; Tag member access (Foo.Bar)
(tag_name
  object: (tag_member) @type
  "." @punctuation.delimiter
  property: (tag_member) @tag)

; Directives (on:click, bind:value, etc.)
(attribute_directive) @keyword
(attribute_name ":" @punctuation.delimiter)
(attribute_identifier) @property
(attribute_modifier) @attribute
(attribute_modifiers "|" @punctuation.delimiter)

; Expressions
(expression) @embedded
(expression_value) @embedded

; Shorthand/spread attributes
(shorthand_attribute content: (_) @variable)

; Curly braces (expression context)
[
  "{"
  "}"
] @punctuation.bracket

"|" @punctuation.delimiter

; Comments inside tag attribute lists
(tag_comment kind: (line_comment) @comment)
(tag_comment kind: (block_comment) @comment)

; Block keywords
[
  "if"
  "each"
  "await"
  "key"
  "snippet"
  "else"
  "html"
  "debug"
  "const"
  "render"
  "attach"
  "as"
] @keyword.control

; Block end keywords ({/if}, {/each}, etc.)
(block_keyword) @keyword.control

; Block delimiters
(block_open) @punctuation.bracket
(block_close) @punctuation.bracket

(shorthand_kind) @keyword.control
(branch_kind) @keyword.control

; If block
(if_block expression: (expression) @embedded)
(else_if_clause expression: (expression_value) @embedded)

; Each block
(each_block expression: (expression) @embedded)
(each_block binding: (pattern) @variable)
(each_block index: (pattern) @variable)
(each_block key: (expression) @embedded)

; Await block
(await_block expression: (expression) @embedded)
(await_branch (pattern) @variable)
(await_block (pattern) @variable)
(orphan_branch (pattern) @variable)

; Key block
(key_block expression: (expression) @embedded)

; Snippet block
(snippet_block name: (snippet_name) @function)
(snippet_parameters parameter: (pattern) @variable)
(snippet_type_parameters) @type

; Malformed blocks
(block_sigil) @keyword.control

; Snippet/render punctuation
[
  "("
  ")"
  ","
] @punctuation.delimiter
