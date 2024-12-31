[
  "syntax"
  "edition"
  "package"
  "option"
  "import"
  "service"
  "rpc"
  "returns"
  "message"
  "map"
  "enum"
  "oneof"
  "repeated"
  "optional"
  "required"
  "reserved"
  "to"
  "stream"
  "extend"
] @keyword

[
  (key_type)
  (type)
  (message_or_enum_type)
] @type.builtin

[
  (enum_name)
  (message_name)
  (service_name)
  (rpc_name)
] @type

[
  (field_name)
  (option_name)
] @variable.other.member
(enum_variant_name) @type.enum.variant

(full_ident) @namespace

(int_lit) @constant.numeric.integer
(float_lit) @constant.numeric.float
(bool) @constant.builtin.boolean
(string) @string

(block_lit) @constant

(comment) @comment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

"=" @operator

";" @punctuation.delimiter
