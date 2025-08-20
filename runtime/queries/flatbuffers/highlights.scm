[
    "include"
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
    (inf_token)
    (nan_token)
] @constant.builtin

[
    (float_constant)
    (int_constant)
] @number

(int_lit) @constant.numeric.integer
(float_lit) @constant.numeric.float

[
    (comment)
    (documentation)
] @comment

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
    namespace_ident: (full_ident) @module)

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
