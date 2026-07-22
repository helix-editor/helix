(comment) @comment

(keyword_interface) @keyword
(keyword_type) @keyword.storage.type
(keyword_method) @keyword.function
(keyword_error) @keyword.storage.type

(interface_name) @string
(method name: (_) @function)
(error name: (_) @type)
(typedef name: (_) @type)
(typeref (name) @type)
(struct_field name: (_) @variable.parameter)
(enum member: (_) @type)

[
  (bool)
  (int)
  (float)
  (string)
  (object)
  (any)
] @type.builtin

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

[
  ","
  ":"
] @punctuation.delimiter

[
  (questionmark)
  (arrow)
] @operator
