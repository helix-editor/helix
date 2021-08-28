[
  "syntax"
  "package"
  "option"
  "import"
  "service"
  "rpc"
  "returns"
  "message"
  "enum"
  "oneof"
  "repeated"
  "reserved"
  "to"
  "stream"
  "extend"
  "optional"
] @keyword

[
  (keyType)
  (type)
] @type.builtin

[
  (mapName)
  (enumName)
  (messageName)
  (extendName)
  (serviceName)
  (rpcName)
] @type

[
  (fieldName)
  (optionName)
] @property
(enumVariantName) @type.enum.variant

(fullIdent) @namespace

[
  (intLit)
  (floatLit)
] @number
(boolLit) @constant.builtin
(strLit) @string

(constant) @constant

(comment) @comment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket