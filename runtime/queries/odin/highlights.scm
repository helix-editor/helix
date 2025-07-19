
; Variables

(identifier) @variable

[
  (calling_convention)
  (tag)
] @keyword.directive

[
  "import" 
] @keyword.control.import

[
  "package"
  "foreign"
  "using"
  "cast"
  "transmute"
  "auto_cast"
] @keyword

[
  "defer"
] @keyword.control

[
  "struct"
  "enum"
  "union"
  "map"
  "bit_set"
  "matrix"
  "bit_field"
] @keyword.storage.type

[
  "proc"
] @keyword.function

[
  "return"
  "or_return"
] @keyword.control.return

[
  "distinct"
  "dynamic"
] @keyword.storage.modifier

[
  "if"
  "else"
  "when"
  "switch"
  "case"
  "where"
  "break"
  "or_break"
  (fallthrough_statement)
] @keyword.control.conditional

((ternary_expression
  [
    "?"
    ":"
    "if"
    "else"
    "when"
  ] @keyword.control.conditional))

[
  "for"
  "do"
  "continue"
  "or_continue"
] @keyword.control.repeat

[
  ":="
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "%%"
  ">"
  ">="
  "<"
  "<="
  "=="
  "!="
  "~="
  "|"
  "~"
  "&"
  "&~"
  "<<"
  ">>"
  "||"
  "&&"
  "!"
  "^"
  ".."
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "||="
  "&&="
  "&~="
  "..="
  "..<"
  "?"
] @operator

[
  "or_else"
  "in"
  "not_in"
] @keyword.operator

[ "{" "}" ] @punctuation.bracket

[ "(" ")" ] @punctuation.bracket

[ "[" "]" ] @punctuation.bracket

[
  "::"
  "->"
  "."
  ","
  ":"
  ";"
] @punctuation.delimiter

[
  "@"
  "$"
] @punctuation.special

(number) @constant.numeric

(float) @constant.numeric.float

(string) @string

(character) @string

(escape_sequence) @constant.character.escape

(boolean) @constant.builtin.boolean

[
  (uninitialized)
  (nil)
] @constant.builtin

((identifier) @variable.builtin
  (#any-of? @variable.builtin "context" "self"))

(((identifier) @type.builtin)
  (#any-of? @type.builtin
    "bool" "byte" "b8" "b16" "b32" "b64"
    "int" "i8" "i16" "i32" "i64" "i128"
    "uint" "u8" "u16" "u32" "u64" "u128" "uintptr"
    "i16le" "i32le" "i64le" "i128le" "u16le" "u32le" "u64le" "u128le"
    "i16be" "i32be" "i64be" "i128be" "u16be" "u32be" "u64be" "u128be"
    "float" "double" "f16" "f32" "f64" "f16le" "f32le" "f64le" "f16be" "f32be" "f64be"
    "complex32" "complex64" "complex128" "complex_float" "complex_double"
    "quaternion64" "quaternion128" "quaternion256"
    "rune" "string" "cstring" "rawptr" "typeid" "any"))

"..." @type.builtin

[
  (comment)
  (block_comment)
] @comment

; Functions

(procedure_declaration (identifier) @function)

(procedure_declaration (identifier) @function (procedure (block)))

(procedure_declaration (identifier) @function (procedure (uninitialized)))

(overloaded_procedure_declaration (identifier) @function)

(call_expression function: (identifier) @function)

(call_expression
  function: (identifier) @function.builtin
  (#any-of? @function.builtin
    "abs" "align_of" "append" "append_elem" "append_elem_string" 
    "append_elems" "append_nothing" "append_soa" "append_soa_elem" 
    "append_soa_elems" "append_string" "assert" "assert_contextless" 
    "assign_at" "assign_at_elem" "assign_at_elem_string" "assign_at_elems" 
    "cap" "card" "clamp" "clear" "clear_dynamic_array" "clear_map" "clear_soa" 
    "complex" "conj" "container_of" "copy" "copy_from_string" "copy_slice" 
    "delete" "delete_cstring" "delete_dynamic_array" "delete_key" 
    "delete_map" "delete_slice" "delete_soa" "delete_string" 
    "expand_values" "free" "free_all" "imag" "init_global_temporary_allocator" 
    "inject_at" "inject_at_elem" "inject_at_elem_string" "inject_at_elems" 
    "jmag" "kmag" "len" "make" "make_dynamic_array" "make_dynamic_array_len" 
    "make_dynamic_array_len_cap" "make_map" "make_multi_pointer" "make_slice" 
    "make_soa" "make_soa_aligned" "make_soa_dynamic_array" 
    "make_soa_dynamic_array_len" "make_soa_dynamic_array_len_cap" 
    "make_soa_slice" "map_insert" "map_upsert" "max" "min" 
    "new_clone" "non_zero_append" "non_zero_append_elem" 
    "non_zero_append_elem_string" "non_zero_append_elems" 
    "non_zero_append_soa_elem" "non_zero_append_soa_elems" 
    "non_zero_resize" "non_zero_resize_dynamic_array" "non_zero_resize_soa" 
    "non_zero_reserve" "non_zero_reserve_dynamic_array" "non_zero_reserve_soa" 
    "offset_of" "offset_of_by_string" "offset_of_member" "offset_of_selector" 
    "ordered_remove" "panic" "panic_contextless" "pop" "pop_front" 
    "pop_front_safe" "pop_safe" "raw_data" "raw_soa_footer_dynamic_array" 
    "raw_soa_footer_slice" "real" "remove_range" "reserve" 
    "reserve_dynamic_array" "reserve_map" "reserve_soa" "resize" 
    "resize_dynamic_array" "resize_soa" "shrink" "shrink_map" 
    "size_of" "soa_unzip" "soa_zip" "swizzle" "type_info_of" "type_of" 
    "typeid_of" "unordered_remove" "unordered_remove_soa" 
    "unimplemented" "unimplemented_contextless"))

; Types

(struct_declaration (identifier) @type "::")

(enum_declaration (identifier) @type "::")

(union_declaration (identifier) @type "::")

(bit_field_declaration (identifier) @type "::")

(const_declaration (identifier) @type "::" [(array_type) (distinct_type) (bit_set_type) (pointer_type)])

(struct . (identifier) @type)

(field_type . (identifier) @keyword.storage.type "." (identifier) @type)

(bit_set_type (identifier) @type ";")

(polymorphic_parameters (identifier) @type)

((identifier) @type
  (#match? @type "^[A-Z][a-z0-9_]+"))

(type (identifier) @type)

; Constants

(member_expression . "." (identifier) @constant)

(enum_declaration "{" (identifier) @constant)

((identifier) @constant
  (#match? @constant "^[A-Z0-9_]*$"))

; Attributes

(attribute (identifier) @attribute "="?)

; Labels

(label_statement (identifier) @label ":")

; Fields

(member_expression "." (identifier) @variable.other.member)
(member_expression
  (identifier) "."
  (call_expression
    function: (identifier) @function.method))

(struct_type "{" (identifier) @variable.other.member)

(struct_field (identifier) @variable.other.member "="?)

(bit_field_declaration (identifier) @variable.other.member)

(field (identifier) @variable.other.member)

; Namespaces

(package_declaration (identifier) @namespace)

(foreign_block (identifier) @namespace)

(using_statement (identifier) @namespace)

(import_declaration (identifier) @namespace)

; Parameters

(parameter (identifier) @variable.parameter ":" "="? (identifier)? @constant)

(default_parameter (identifier) @variable.parameter ":=")

(named_type (identifier) @variable.parameter)

(call_expression argument: (identifier) @variable.parameter "=")

(procedure_type (parameters (parameter (identifier) @variable.parameter)))
