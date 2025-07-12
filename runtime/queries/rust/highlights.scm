; -------
; Basic identifiers
; -------

; We do not style ? as an operator on purpose as it allows styling ? differently, as many highlighters do. @operator.special might have been a better scope, but @special is already documented so the change would break themes (including the intent of the default theme)
"?" @special

(type_identifier) @type
(identifier) @variable
(field_identifier) @variable.other.member

; -------
; Operators
; -------

[
  "*"
  "'"
  "->"
  "=>"
  "<="
  "="
  "=="
  "!"
  "!="
  "%"
  "%="
  "&"
  "&="
  "&&"
  "|"
  "|="
  "||"
  "^"
  "^="
  "*"
  "*="
  "-"
  "-="
  "+"
  "+="
  "/"
  "/="
  ">"
  "<"
  ">="
  ">>"
  "<<"
  ">>="
  "<<="
  "@"
  ".."
  "..="
  "'"
] @operator

; -------
; Paths
; -------

(use_declaration
  argument: (identifier) @namespace)
(use_wildcard
  (identifier) @namespace)
(extern_crate_declaration
  name: (identifier) @namespace
  alias: (identifier)? @namespace)
(mod_item
  name: (identifier) @namespace)
(scoped_use_list
  path: (identifier)? @namespace)
(use_list
  (identifier) @namespace)
(use_as_clause
  path: (identifier)? @namespace
  alias: (identifier) @namespace)

; -------
; Types
; -------

(type_parameters
  (type_identifier) @type.parameter)
(constrained_type_parameter
  left: (type_identifier) @type.parameter)
(optional_type_parameter
  name: (type_identifier) @type.parameter)
((type_arguments (type_identifier) @constant)
 (#match? @constant "^[A-Z_]+$"))
(type_arguments (type_identifier) @type)
; `_` in `(_, _)`
(tuple_struct_pattern "_" @comment.unused)
; `_` in `Vec<_>`
((type_arguments (type_identifier) @comment.unused)
 (#eq? @comment.unused "_"))
; `_` in `Rc<[_]>`
((array_type (type_identifier) @comment.unused)
 (#eq? @comment.unused "_"))

; ---
; Primitives
; ---

(escape_sequence) @constant.character.escape
(primitive_type) @type.builtin
(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(char_literal) @constant.character
[
  (string_literal)
  (raw_string_literal)
] @string

; -------
; Comments
; -------

(line_comment) @comment.line
(block_comment) @comment.block

; Doc Comments
(line_comment
  (outer_doc_comment_marker "/" @comment.line.documentation)
  (doc_comment)) @comment.line.documentation
(line_comment
  (inner_doc_comment_marker "!" @comment.line.documentation)
  (doc_comment)) @comment.line.documentation

(block_comment
  (outer_doc_comment_marker) @comment.block.documentation
  (doc_comment) "*/" @comment.block.documentation) @comment.block.documentation
(block_comment
  (inner_doc_comment_marker) @comment.block.documentation
  (doc_comment) "*/" @comment.block.documentation) @comment.block.documentation

; ---
; Extraneous
; ---

(self) @variable.builtin

(field_initializer
  (field_identifier) @variable.other.member)
(shorthand_field_initializer
  (identifier) @variable.other.member)
(shorthand_field_identifier) @variable.other.member

(lifetime
  "'" @label
  (identifier) @label)
(label
  "'" @label
  (identifier) @label)

; ---
; Punctuation
; ---

[
  "::"
  "."
  ";"
  ","
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "#"
] @punctuation.bracket
(type_arguments
  [
    "<"
    ">"
  ] @punctuation.bracket)
(type_parameters
  [
    "<"
    ">"
  ] @punctuation.bracket)
(for_lifetimes ["<" ">"] @punctuation.bracket)
(closure_parameters
  "|" @punctuation.bracket)
(bracketed_type ["<" ">"] @punctuation.bracket)

; ---
; Variables
; ---

(let_declaration
  pattern: [
    ((identifier) @variable)
    ((tuple_pattern
      (identifier) @variable))
  ])
  
; It needs to be anonymous to not conflict with `call_expression` further below. 
(_
 value: (field_expression
  value: (identifier)? @variable
  field: (field_identifier) @variable.other.member))

(parameter
	pattern: (identifier) @variable.parameter)
(closure_parameters
	(identifier) @variable.parameter)

; -------
; Keywords
; -------

(for_expression
  "for" @keyword.control.repeat)
(gen_block "gen" @keyword.control)

"in" @keyword.control

[
  "match"
  "if"
  "else"
  "try"
] @keyword.control.conditional

[
  "while"
  "loop"
] @keyword.control.repeat

[
  "break"
  "continue"
  "return"
  "await"
  "yield"
] @keyword.control.return

"use" @keyword.control.import
(mod_item "mod" @keyword.control.import !body)
(use_as_clause "as" @keyword.control.import)

(type_cast_expression "as" @keyword.operator)

((generic_type
    type: (type_identifier) @keyword)
 (#eq? @keyword "use"))

[
  (crate)
  (super)
  "as"
  "pub"
  "mod"
  "extern"

  "impl"
  "where"
  "trait"
  "for"

  "default"
  "async"
] @keyword

[
  "struct"
  "enum"
  "union"
  "type"
] @keyword.storage.type

"let" @keyword.storage
"fn" @keyword.function
"unsafe" @keyword.special
"macro_rules!" @function.macro

(mutable_specifier) @keyword.storage.modifier.mut

(reference_type "&" @keyword.storage.modifier.ref)
(self_parameter "&" @keyword.storage.modifier.ref)

[
  "static"
  "const"
  "raw"
  "ref"
  "move"
  "dyn"
] @keyword.storage.modifier

; TODO: variable.mut to highlight mutable identifiers via locals.scm

; ---
; Remaining Paths
; ---

(scoped_identifier
  path: (identifier)? @namespace
  name: (identifier) @namespace)
(scoped_type_identifier
  path: (identifier) @namespace)

; -------
; Functions
; -------

; In here, `bar` is a function, as it is equal to a closure:
;
; let bar = || 4;
(let_declaration
  pattern: (identifier) @function
  value: (closure_expression))

; highlight `baz` in `any_function(foo::bar::baz)` as function
; This generically works for an unlimited number of path segments:
;
; - `f(foo::bar)`
; - `f(foo::bar::baz)`
; - `f(foo::bar::baz::quux)`
;
; We know that in the above examples, the last component of each path is a function
; as the only other valid thing (following Rust naming conventions) would be a module at
; that position, however you cannot pass modules as arguments
(call_expression
  function: _
  arguments: (arguments
    (scoped_identifier
      path: _
      name: (identifier) @function)))

; Special handling for point-free functions that are not part of a path
; but are just passed as variables to some "well-known"
; methods, which are known to take a closure as the first argument
; 
; For example, `foo` in `a.map(foo)` is a @function
(call_expression
  function: (field_expression
    value: _
    field: (field_identifier) @_method_name)
  arguments:
    ; first argument is `@function`
    (arguments
      .
      (identifier) @function)
  (#any-of? @_method_name
  ; methods on `core::iter::Iterator`
  "map" "map_while" "filter_map" "flat_map" "map_windows"
  "try_for_each" "for_each"
  "is_sorted_by" "is_sorted_by_key"
  "all" "any" "reduce" "try_reduce" 
  "find" "find_map" "try_find" "position" "rposition"
  "max_by" "max_by_key" "min_by" "min_by_key"
  "filter" "inspect" "intersperse_with"
  "partition" "partition_in_place" "is_partitioned"
  "skip_while" "take_while"
  
  ; methods on `Option`
  "and_then" "is_some_and" "is_none_or" "unwrap_or_else"
  "ok_or_else" "or_else" "get_or_insert_with" "take_if"
  "map_or_else" ; NOTE: both arguments are closures, so it is here and also in the block to
                ; highlight the 2nd argument
  
  ; methods on `Result
  "map_err" "inspect_err"

  ; methods on `Entry`
  "or_insert_with" "or_insert_with_key" "and_modify"

  ; method on `bool
  "then"

  ; method on `Vec`
  "chunk_by_mut" "split" "split_mut" "split_inclusive" "split_inclusive_mut"
  "rsplit" "rsplit_mut" "binary_search_by"
  "sort_unstable_by" "sort_unstable_by_key" "partition_dedup_by"
  "partition_dedup_by_key" "fill_with" "partition_point" "sort_by"
  "sort_by_key"

  ; methods on `HashMap`
  "extract_if" "retain"

  ; methods on `itertools::Itertools`
  "batching" "chunk_by" "group_by" "map_ok"
  "filter_ok" "filter_map_ok" "process_results"
  "kmerge_by" "coalesce" "dedup_by" "dedup_by_with_count"
  "duplicates_by" "unique_by" "peeking_take_while"
  "take_while_ref" "take_while_inclusive" "positions"
  "update" "find_position" "find_or_last" "find_or_first"
  "fold1" "tree_reduce" "tree_fold1" "partition_map"
  "into_group_map_by" "into_grouping_map_by"
  "min_set_by" "min_set_by_key" "max_set_by" "max_set_by_key"
  "minmax_by_key" "minmax_by" "position_max_by_key"
  "position_max_by" "position_min_by_key" "position_min_by"
  "position_minmax_by_key" "position_minmax_by"
  "sorted_unstable_by" "sorted_unstable_by_key" "sorted_by"
  "sorted_by_key" "sorted_by_cached_key"

  ; method on `core::iter::Peekable`
  "next_if"

  ; methods on `rayon::ParallelIterator`
  ;
  ; note: some of these methods are also
  ; present in the 2nd argument highlights, because
  ; those methods take a closure both as a 1st and 2nd arg
  "for_each_init" "try_for_each_init" "map_init"
  "update"
  "flat_map_iter" "reduce_with" "try_reduce"
  "try_reduce_with" "fold_with" "try_fold_with"
  "find_any" "find_first" "find_last" "find_map_any"
  "find_map_first" "find_map_last"
  "take_any_while" "skip_any_while"

  ; method on `tap::Pipe`
  "pipe" "pipe_ref" "pipe_ref_mut" "pipe_borrow" "pipe_deref_mut"
  "pipe_borrow_mut" "pipe_as_ref" "pipe_as_mut" "pipe_deref"))

; Here, we do something similar to the above except for
; methods that take a closure as a 2nd argument instead of the first
(call_expression
  function: (field_expression
    value: _
    field: (field_identifier) @_method_name)
  arguments: 
    ; handles `a.map_or_else(..., foo)`
    (arguments
      ; first argument is ignored
      .
      ; second argument is @function
      (_)
      .
      (identifier) @function)
  (#any-of? @_method_name
  ; methods on `Option`
  "map_or_else" "zip_with"

  ; methods on `Iterator`
  "try_fold" "scan" "fold" "cmp_by" "partial_cmp_by" "eq_by"

  ; methods on `rayon::ParallelIterator`
  "for_each_with" "for_each_init" "try_for_each_with" "try_for_each_init"
  "map_with" "map_init" "try_reduce" "fold_with" "try_fold_with"
    
  ; methods on `Vec`
  "splitn" "splitn_mut" "rsplitn" "rsplitn_mut" "split_once"
  "rsplit_once" "binary_search_by_key" "select_nth_unstable_by"
  "select_nth_unstable_by_key"
  ; methods on `Itertools`
  "k_largest_by" "k_largest_by_key" "k_largest_relaxed_by"
  "k_largest_relaxed_by_key"
  "k_smallest_by" "k_smallest_by_key" "k_smallest_relaxed_by" "k_smallest_relaxed_by_key"
  "fold_while" "fold_ok" "fold_options" "merge_by" "merge_join_by" "pad_using" "format_with"))

(call_expression
  function: [
    ((identifier) @function)
    (scoped_identifier
      name: (identifier) @function)
    (field_expression
      field: (field_identifier) @function)
  ])
(generic_function
  function: [
    ((identifier) @function)
    (scoped_identifier
      name: (identifier) @function)
    (field_expression
      field: (field_identifier) @function.method)
  ])

(function_item
  name: (identifier) @function)

(function_signature_item
  name: (identifier) @function)

; -------
; Guess Other Types
; -------
; Other PascalCase identifiers are assumed to be structs.

((identifier) @type
  (#match? @type "^[A-Z]"))

(never_type "!" @type)

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

; ---
; PascalCase identifiers in call_expressions (e.g. `Ok()`)
; are assumed to be enum constructors.
; ---

(call_expression
  function: [
    ((identifier) @constructor
      (#match? @constructor "^[A-Z]"))
    (scoped_identifier
      name: ((identifier) @constructor
        (#match? @constructor "^[A-Z]")))
  ])

; ---
; PascalCase identifiers under a path which is also PascalCase
; are assumed to be constructors if they have methods or fields.
; ---

(field_expression
  value: (scoped_identifier
    path: [
      (identifier) @type
      (scoped_identifier
        name: (identifier) @type)
    ]
    name: (identifier) @constructor
      (#match? @type "^[A-Z]")
      (#match? @constructor "^[A-Z]")))

(enum_variant (identifier) @type.enum.variant)


; -------
; Constructors
; -------
; TODO: this is largely guesswork, remove it once we get actual info from locals.scm or r-a

(struct_expression
  name: (type_identifier) @constructor)

(tuple_struct_pattern
  type: [
    (identifier) @constructor
    (scoped_identifier
      name: (identifier) @constructor)
  ])
(struct_pattern
  type: [
    ((type_identifier) @constructor)
    (scoped_type_identifier
      name: (type_identifier) @constructor)
  ])
(match_pattern
  ((identifier) @constructor) (#match? @constructor "^[A-Z]"))
(or_pattern
  ((identifier) @constructor)
  ((identifier) @constructor)
  (#match? @constructor "^[A-Z]"))

; ---
; Macros
; ---

(attribute
  (identifier) @function.macro)
(inner_attribute_item "!" @punctuation)
(attribute
  [
    (identifier) @function.macro
    (scoped_identifier
      name: (identifier) @function.macro)
  ]
  (token_tree (identifier) @function.macro)?)

(inner_attribute_item) @attribute

(macro_definition
  name: (identifier) @function.macro)
(macro_invocation
  macro: [
    ((identifier) @function.macro)
    (scoped_identifier
      name: (identifier) @function.macro)
  ]
  "!" @function.macro)

(metavariable) @variable.parameter
(fragment_specifier) @type

(attribute
  (identifier) @special
  arguments: (token_tree (identifier) @type)
  (#eq? @special "derive")
)

(token_repetition_pattern) @punctuation.delimiter
(token_repetition_pattern [")" "(" "$"] @punctuation.special)
(token_repetition_pattern "?" @operator)

; ---
; Prelude
; ---

((identifier) @type.enum.variant.builtin
 (#any-of? @type.enum.variant.builtin "Some" "None" "Ok" "Err"))


(call_expression
  (identifier) @function.builtin
  (#any-of? @function.builtin
    "drop"
    "size_of"
    "size_of_val"
    "align_of"
    "align_of_val"))

((type_identifier) @type.builtin
 (#any-of?
    @type.builtin
    "Send"
    "Sized"
    "Sync"
    "Unpin"
    "Drop"
    "Fn"
    "FnMut"
    "FnOnce"
    "AsMut"
    "AsRef"
    "From"
    "Into"
    "DoubleEndedIterator"
    "ExactSizeIterator"
    "Extend"
    "IntoIterator"
    "Iterator"
    "Option"
    "Result"
    "Clone"
    "Copy"
    "Debug"
    "Default"
    "Eq"
    "Hash"
    "Ord"
    "PartialEq"
    "PartialOrd"
    "ToOwned"
    "Box"
    "String"
    "ToString"
    "Vec"
    "FromIterator"
    "TryFrom"
    "TryInto"))
