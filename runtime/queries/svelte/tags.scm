; Snippet declarations
(snippet_block
  name: (snippet_name) @name) @definition.function

; Component references
((tag_name) @name @reference.class
  (#match? @name "^[A-Z]"))

(tag_name
  object: (tag_member) @name
  property: (tag_member) @name) @reference.class
