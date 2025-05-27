;; Primitives
(comment) @comment
(nat) @constant.numeric
(unit) @constant.builtin
(literal_char) @constant.character
(literal_text) @string
(literal_boolean) @constant.builtin.boolean

;; Keywords
[
  (kw_forall)
  (kw_equals)
  (do)
  (kw_let)
  (ability)
  (where)
] @keyword

(kw_let) @keyword.function
(type_kw) @keyword.storage.modifier
(structural) @keyword.storage.modifier
("use") @keyword.control.import
(unique) @keyword.storage.modifier

[
  (operator)
  (pipe)
  (arrow_symbol)
  (or)
  (and)
] @operator

[
  "if"
  "else"
  "then"
  (match)
  (with)
  (cases)
] @keyword.control.conditional

(blank_pattern) @variable.builtin

(pattern) @variable

(use_clause) @keyword.import

;; Types
(record_field
  (field_name) @variable.other.member
  type: (regular_identifier) @type)

(type_name) @type

(type_declaration
  (regular_identifier) @type.enum.variant)

(ability_name
  (path)? @namespace
  (regular_identifier) @type)

(ability_declaration
  (ability_name) @type
  (type_argument) @variable.parameter)

(type_constructor) @constructor

(constructor
  (constructor_name) @constructor)

(constructor
  type: (regular_identifier) @type)

(effect
  (regular_identifier) @special) ; NOTE: an effect is a special type

; Namespaces
(path) @namespace

(namespace) @namespace

; Terms
(type_signature
  term_name: (path) @namespace
  term_name: (regular_identifier) @variable)

(type_signature
  term_name: (regular_identifier) @variable)

(term_type) @type

(term_definition
  name: (path) @namespace)

(term_definition
  name: (regular_identifier) @variable)

(term_definition
  param: (regular_identifier) @variable.parameter)

;; Punctuation
[
  (type_signature_colon)
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

(watch_expression) @keyword.directive

(test_watch_expression) @keyword.directive
