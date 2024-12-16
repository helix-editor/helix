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
  (type_kw)
  (kw_equals)
  (do)
  (ability)
  (where)
] @keyword

(kw_let) @keyword.function
(type_kw) @keyword.storage.type
(unique) @keyword.storage.modifier
(structural) @keyword.storage.modifier
("use") @keyword.control.import


[
  (type_constructor)
] @constructor

[
  (operator)
  (pipe)
  (arrow_symbol)
  (">")
  (or)
  (and)
  (bang)
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

;; Types
(record_field name: (wordy_id) @variable.other.member type: (_) @type)
(type_constructor (type_name (wordy_id) @constructor))
(ability_declaration type_name: (wordy_id) @type type_arg: (wordy_id) @variable.parameter)
(effect (wordy_id) @special) ;; NOTE: an effect is just like a type, but in signature we special case it

;; Namespaces
(path) @namespace
(namespace) @namespace

;; Terms
(type_signature term_name: (path)? @variable term_name: (wordy_id) @variable)
(type_signature (wordy_id) @type)
(type_signature (term_type(delayed(wordy_id))) @type)

(term_definition param: (wordy_id) @variable.parameter)

(function_application function_name: (path)? function_name: (wordy_id) @function)

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

(test_watch_expression (wordy_id) @keyword.directive)
