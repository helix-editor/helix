; Concerto Language - Syntax Highlighting Queries (Helix)
; =======================================================
; Helix-specific capture names. For use in helix-editor/helix at
; runtime/queries/concerto/highlights.scm
;
; Precedence: later patterns override earlier ones (last match wins).

; Keywords
; --------

; Declaration keywords
[
  "concept"
  "asset"
  "participant"
  "transaction"
  "event"
  "enum"
  "scalar"
  "map"
] @keyword.storage.type

[
  "namespace"
  "import"
  "from"
] @keyword.control.import

[
  "extends"
] @keyword

[
  "abstract"
] @keyword.storage.modifier

[
  "identified"
  "by"
] @keyword

[
  "optional"
] @keyword.storage.modifier

[
  "concerto"
  "version"
] @keyword

[
  "default"
] @keyword

[
  "regex"
  "range"
  "length"
] @keyword

[
  "as"
] @keyword

; Primitive type keywords
[
  "String"
  "Boolean"
  "DateTime"
  "Integer"
  "Long"
  "Double"
] @type.builtin

(primitive_type) @type.builtin

; Boolean literals
(boolean_literal) @constant.builtin.boolean

; Comments
; --------
(line_comment) @comment.line
(block_comment) @comment.block

; Strings
; -------
(string_literal) @string
(escape_sequence) @constant.character.escape

; Numbers
; -------
(signed_integer) @constant.numeric
(signed_real) @constant.numeric
(signed_number) @constant.numeric

; Regex
; -----
(regex_literal) @string.regexp

; Decorators
; ----------
(decorator
  "@" @attribute
  name: (identifier) @attribute)

; Namespace and imports
; --------------------
(namespace_path) @namespace

(import_path) @namespace

(uri) @string.special

; Version
; -------
(concerto_version
  (string_literal) @string.special)

; Type identifiers (in type position)
; -----------------------------------
(concept_declaration
  name: (type_identifier) @type)

(asset_declaration
  name: (type_identifier) @type)

(participant_declaration
  name: (type_identifier) @type)

(transaction_declaration
  name: (type_identifier) @type)

(event_declaration
  name: (type_identifier) @type)

(enum_declaration
  name: (type_identifier) @type)

(scalar_declaration
  name: (type_identifier) @type)

(map_declaration
  name: (type_identifier) @type)

(extends_clause
  (type_identifier) @type)

; Field type references
(object_field
  type: (type_identifier) @type)

(relationship_field
  type: (type_identifier) @type)

; Map type references
(map_key_type
  type: (type_identifier) @type)

(map_value_property
  type: (type_identifier) @type)

(map_value_relationship
  type: (type_identifier) @type)

; Decorator identifier references
(decorator_identifier_ref
  (type_identifier) @type)

; Import type name
(import_single
  type: (identifier) @type)

(type_list_item
  (identifier) @type)

(aliased_type
  original: (identifier) @type
  alias: (identifier) @type)

; Field/property names
; --------------------
(string_field
  name: (identifier) @variable.other.member)

(boolean_field
  name: (identifier) @variable.other.member)

(datetime_field
  name: (identifier) @variable.other.member)

(integer_field
  name: (identifier) @variable.other.member)

(long_field
  name: (identifier) @variable.other.member)

(double_field
  name: (identifier) @variable.other.member)

(object_field
  name: (identifier) @variable.other.member)

(relationship_field
  name: (identifier) @variable.other.member)

(enum_property
  name: (identifier) @variable.other.member)

; Identified by field name
(identified_by
  field: (identifier) @variable.other.member)

; Relationship arrow
"-->" @operator

; Property indicator
"o" @punctuation.special

; Array indicator
(array_indicator) @punctuation.bracket

; Wildcards in imports
"*" @operator

; Delimiters
; ----------
"{" @punctuation.bracket
"}" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"," @punctuation.delimiter
"." @punctuation.delimiter
"=" @operator
