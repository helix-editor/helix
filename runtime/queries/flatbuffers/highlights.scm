[
    "namespace"
    "attribute"
    "table"
    "struct"
    "union"
    "enum"
    "root_type"
    "rpc_service"
    "file_extension"
    "file_identifier"
] @keyword

"include" @keyword.control.import

[
  ";"
  "."
  ","
] @punctuation.delimiter

(type) @type.builtin
(string_constant) @string

[
    (true)
    (false)
] @constant.builtin.boolean

[
    (inf_token)
    (nan_token)
] @constant.builtin

[
    (int_lit)
    (int_constant)
] @constant.numeric.integer

[
    (float_lit)
    (float_constant)
] @constant.numeric.float


(comment) @comment
(documentation) @comment.line.documentation

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

[
    (metadata)
] @attribute

(attribute_decl
  attribute_name: (identifier) @string)

(namespace_decl
    namespace_ident: (full_ident) @namespace)

(type_decl
    table_or_struct_name: (identifier) @type)

(enum_decl
    enum_name: (identifier) @type)

(enum_val_decl
    enum_key: (identifier) @type)

(union_decl
    union_name: (identifier) @type)

(root_decl
    root_type_ident: (identifier) @type)

(rpc_decl
    rpc_name: (identifier) @type)

(rpc_method
    rpc_method_name: (identifier) @function
    rpc_parameter: (identifier) @variable.parameter
    rpc_return_type: (identifier) @type)
