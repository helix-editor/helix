(line_comment) @comment.line
(block_comment) @comment.block
(ty (ident) @type)

(item_type name: (ident) @type)
(item_record name: (ident) @type)
(item_variant name: (ident) @type)
(item_flags name: (ident) @type)
(item_enum name: (ident) @type)
(item_union name: (ident) @type)
(item_resource name: (ident) @type)

(item_use from: (ident) @namespace)
(use_item name: (ident) @type)
(item_func name: (ident) @function)
(method name: (ident) @function.method)
(fields (named_ty name: (ident) @variable.other.member))
(input (args (named_ty name: (ident) @variable.parameter)))
(output (args (named_ty name: (ident) @variable.other.member)))
(flags (ident) @constant)
(enum_items (ident) @constant)
(variant_item tag: (ident) @type.enum.variant)

[
  (unit)

  "u8" "u16" "u32" "u64"
  "s8" "s16" "s32" "s64"
  "float32" "float64"
  "char" "bool" "string"
] @type.builtin

[
  "list"
  "option"
  "result"
  "tuple"
  "future"
  "stream"
] @function.macro

[ "," ":" ] @punctuation.delimiter
[ "(" ")" "{" "}" "<" ">" ] @punctuation.bracket
[ "=" "->" ] @operator

[
  "record"
  "flags"
  "variant"
  "enum"
  "union"
  "type"
  "resource"
] @keyword.storage.type

"func" @keyword

[
  "static"
] @keyword.storage.modifier

[
  (star)
  "use"
  "as"
  "from"
] @keyword.control.import
