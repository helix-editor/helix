;; Literals
(integer_literal) @constant.numeric.integer
(natural_literal) @constant.numeric.integer
(double_literal) @constant.numeric.float
(boolean_literal) @constant.builtin.boolean
(builtin "None") @constant.builtin

;; Text
(text_literal) @string
(interpolation "}" @string)
(double_quote_escaped) @constant.character.escape
(single_quote_escaped) @constant.character.escape

;; Imports
(local_import) @string.special.path
(http_import) @string.special.url
(env_import) @keyword
(env_variable) @string.special
(import_hash) @string.special
(missing_import) @keyword.control.import
[ (import_as_location) (import_as_text) ] @type

;; Comments
(block_comment) @comment.block
(line_comment) @comment.line

;; Types
([
  (let_binding (label) @type)
  (union_type_entry (label) @type)
] (#match? @type "^[A-Z]"))
((primitive_expression
  (identifier (label) @type)
  (selector (label) @type)?) @whole_identifier
  (#match? @whole_identifier "(?:^|\\.)[A-Z][^.]*$"))

;; Variables
(identifier [
  (label) @variable
  (de_bruijn_index) @operator
])
(let_binding label: (label) @variable)
(lambda_expression label: (label) @variable.parameter)
(record_literal_entry (label) @variable.other.member)
(record_type_entry (label) @variable.other.member)
(selector) @variable.other.member

;; Keywords
[
  "let"
  "in"
  "assert"
] @keyword
[
  "using"
  "as"
  "with"
] @keyword.operator

;; Operators
[
  (type_operator)
  (assign_operator)
  (lambda_operator)
  (arrow_operator)
  (infix_operator)
  (completion_operator)
  (assert_operator)
  (forall_operator)
  (empty_record_literal)
] @operator

;; Builtins
(builtin_function) @function.builtin
(builtin [
  "Bool"
  "Optional"
  "Natural"
  "Integer"
  "Double"
  "Text"
  "Date"
  "Time"
  "TimeZone"
  "List"
  "Type"
  "Kind"
  "Sort"
] @type.builtin)

;; Punctuation
[ "," "|" ] @punctuation.delimiter
(selector_dot) @punctuation.delimiter
[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
  "<"
  ">"
] @punctuation.bracket

;; Conditionals
[
  "if"
  "then"
  "else"
] @keyword.control.conditional
