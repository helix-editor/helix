;; Tree-sitter highlight queries for strictdoc

;; —————————————————————————————————————————
;; Keywords “blok” del documento
;; —————————————————————————————————————————
[
  "[DOCUMENT]"
  "[DOCUMENT_FROM_FILE]"
  "[GRAMMAR]"
  "[/SECTION]"
  "[SECTION]"
  (sdoc_node_opening)
  (sdoc_composite_node_opening)
  (sdoc_composite_node_closing)
  (sdoc_composite_node_type_name)
] @keyword

[
  "AUTO_LEVELS"
  "CLASSIFICATION"
  "DATE"
  "DEFAULT_VIEW"
  "ELEMENTS"
  "ENABLE_MID"
  "FIELDS"
  "FILE"
  "FORMAT"
  "IMPORT_FROM_FILE"
  "IS_COMPOSITE"
  "LAYOUT"
  "LEVEL"
  "MARKUP"
  "METADATA"
  "MID"
  "NAME"
  "NODE_IN_TOC"
  "OBJECT_TYPE"
  "OPTIONS"
  "PLACEMENT"
  "PREFIX"
  "PROPERTIES"
  "REQ_PREFIX"
  "REQUIRED"
  "REQUIREMENT_IN_TOC"
  "REQUIREMENT_STYLE"
  "ROLE"
  "ROOT"
  "TAG"
  "TITLE"
  "TYPE"
  "UID"
  "VALUE"
  "VERSION"
  "VIEW_STYLE"
  "VISIBLE_FIELDS"
] @type.builtin

;; Operators
[
  (multiline_opening_token)
  (multiline_closing_token)
] @operator

;; Punctuation
[
  ":" @punctuation.delimiter
  "," @punctuation.delimiter
  "-" @punctuation.delimiter
]

;; Boolean literals
(boolean_choice) @constant.builtin.boolean

;; Requirement types and file formats

;; Config option values
[
  "Child"
  "Default"
  "File"
  "HTML"
  "Inline"
  "Narrative"
  "Off"
  "On"
  "Parent"
  "Plain"
  "RST"
  "Simple"
  "Table"
  "Text"
  "Website"
  "Zebra"
] @constant.builtin


;; Strings
(single_line_string) @string
[ (uid_string) (req_reference_value_id) ] @string.special.symbol
(date) @string.special

;; Fields
(document_custom_metadata_key) @type.parameter
[ "RELATIONS" (field_name) ] @variable.other.member
(choice_option) @variable.parameter

;; Anchors and links
(anchor) @label
(inline_link) @string.special.url

[
 (role_id)
] @variable
