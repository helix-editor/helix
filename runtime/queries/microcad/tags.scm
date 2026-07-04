; function definitions
(
  (function_definition
    doc: (doc_comment)* @doc
    name: (identifier) @name) @definition.function
  (#strip! @doc "^///\\s*")
)

; module definitions
(
  (module_definition
    doc: (doc_comment)* @doc
    name: (identifier) @name) @definition.module
  (#strip! @doc "^///\\s*")
)

; workbench definitions
(
  (workbench_definition
    doc: (doc_comment)* @doc
    name: (identifier) @name) @definition.function
  (#strip! @doc "^///\\s*")
)

; references

(call
    method: (qualified_name) @name) @reference.call

(method_call
    method: (qualified_name) @name) @reference.call

