(ty
  (id)) @type

(decl_head
  (id) @module)

(version) @string.special

(use_path
  [
    "@"
    "/"
  ] @punctuation.delimiter)

(decl_head
  [
    "@"
    "/"
  ] @punctuation.delimiter)

; feature gates with leading `@`
(_
  .
  "@" @punctuation.special
  .
  [
    "since"
    "unstable"
    "deprecated"
  ] @attribute.builtin)

(unstable_gate
  feature: (id) @string)

(world_item
  name: (id) @module)

(interface_item
  name: (id) @module)

(import_item
  name: (id) @module
  (extern_type
    (body)))

(import_item
  name: (id) @function
  (extern_type
    (func_type)))

(export_item
  name: (id) @module
  (extern_type
    (body)))

(export_item
  name: (id) @function
  (extern_type
    (func_type)))

(type_item
  alias: (id) @type.definition)

(func_item
  name: (id) @function.method)

(handle
  (id) @type)

(named_type
  name: (id) @variable.parameter)

(record_item
  name: (id) @type)

(record_field
  name: (id) @variable.member)

(flags_items
  name: (id) @type)

(flags_field) @variable.member

(variant_items
  name: (id) @type)

(variant_case
  name: (id) @constant)

(enum_items
  name: (id) @type)

(enum_case) @constant

(resource_item
  name: (id) @type)

(resource_method
  (id) @function.method)

(resource_method
  "constructor" @constructor)

(toplevel_use_item
  "use" @keyword.import)

(toplevel_use_item
  alias: (id) @module)

(use_item
  "use" @keyword.import)

(use_path
  (id) @module)

(alias_item
  (id) @module)

(use_names_item
  (id) @module)

"func" @keyword.function

(external_id
  "@" @punctuation.special
  "external-id" @attribute.builtin
  id: (string_literal) @string)


[
  "type"
  "interface"
  "world"
  "package"
  "resource"
  "record"
  "enum"
  "flags"
  "variant"
] @keyword.type

"static" @keyword.modifier

"async" @keyword.coroutine

(uint) @constant

[
  "include"
  "import"
  "export"
  "as"
  "with"
] @keyword.import

[
  "u8"
  "u16"
  "u32"
  "u64"
  "s8"
  "s16"
  "s32"
  "s64"
  "f32"
  "f64"
  "char"
  "bool"
  "string"
] @type.builtin

[
  "tuple"
  "list"
  "option"
  "result"
  "map"
  "borrow"
  "future"
  "stream"
] @type

"_" @variable.parameter.builtin

[
  ";"
  ":"
  ","
  "."
  "->"
] @punctuation.delimiter

(use_path
  "/" @punctuation.delimiter)

[
  "{"
  "}"
  "("
  ")"
  ">"
  "<"
] @punctuation.bracket

"=" @operator

[
  (line_comment)
  (block_comment)
] @comment @spell

(line_comment
  (doc_comment)) @comment.documentation

(block_comment
  (doc_comment)) @comment.documentation
