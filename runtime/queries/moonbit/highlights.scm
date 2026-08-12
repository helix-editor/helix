; Packages

(package_identifier) @namespace

; Variables

(positional_parameter (lowercase_identifier) @variable.parameter)
(labelled_parameter (label (lowercase_identifier)) @variable.parameter)
(optional_parameter (optional_label (lowercase_identifier)) @variable.parameter)
(optional_parameter_with_default (label (lowercase_identifier)) @variable.parameter)
((positional_parameter (lowercase_identifier) @variable.builtin)
 (#any-of? @variable.builtin "self"))
((labelled_parameter (label (lowercase_identifier)) @variable.builtin)
 (#any-of? @variable.builtin "self"))
((optional_parameter (optional_label (lowercase_identifier)) @variable.builtin)
 (#any-of? @variable.builtin "self"))
((optional_parameter_with_default (label (lowercase_identifier)) @variable.builtin)
 (#any-of? @variable.builtin "self"))

(tuple_pattern (lowercase_identifier) @variable)
(constructor_pattern_argument . (lowercase_identifier) @variable .)
(constructor_pattern_argument "=" (lowercase_identifier) @variable)
(constructor_pattern_argument (label (lowercase_identifier) @variable))
(case_clause (lowercase_identifier) @variable "=>")
(matrix_case_clause (lowercase_identifier) @variable "=>")
(let_expression (lowercase_identifier) @variable)

(qualified_identifier (lowercase_identifier) @variable)
((qualified_identifier (lowercase_identifier) @variable.builtin)
 (#any-of? @variable.builtin "self"))
(qualified_identifier (dot_lowercase_identifier) @variable)

(value_definition (lowercase_identifier) @variable)
(let_mut_expression (lowercase_identifier) @variable)
(for_in_expression "for" (lowercase_identifier) @variable "in")
(for_binder (lowercase_identifier) @variable)
(package_statement_identifier) @variable
(package_assignment_statement
  name: (package_statement_identifier) @variable)

; Constructors

(enum_constructor) @constructor
(constructor_expression (uppercase_identifier) @constructor)
(constructor_expression (dot_uppercase_identifier) @constructor)

; Constants

(const_definition (uppercase_identifier) @constant)
((constructor_expression (uppercase_identifier) @constant)
 (#match? @constant "^[A-Z][A-Z_]+$"))
((constructor_expression (dot_uppercase_identifier) @constant)
 (#match? @constant "^\.[A-Z][A-Z_]+$"))

; Types

(type_identifier) @type
(qualified_type_identifier) @type

(enum_definition (identifier) @type)
(struct_definition (identifier) @type)
(tuple_struct_definition (identifier) @type)
(type_definition (identifier) @type)
(trait_definition (identifier) @type)
(type_alias_targets (identifier) @type)
(type_alias_targets (dot_identifier) @type)
(type_alias_target (identifier) @type)
(error_type_definition (identifier) @type)
(trait_alias_targets (identifier) @type)
(trait_alias_targets (dot_identifier) @type)
(trait_alias_target (identifier) @type)

((qualified_type_identifier) @type.builtin
 (#any-of? @type.builtin
           "Unit" "Bool" "Byte"
           "Int16" "UInt16" "Int" "UInt" "Int64" "UInt64"
           "Float" "Double"
           "FixedArray" "Array" "Bytes" "String" "Error" "Self"))

((qualified_type_identifier) @type.builtin
 (#any-of? @type.builtin
           "Eq" "Compare" "Hash" "Show" "Default" "ToJson" "FromJson"))

; Fields

(struct_field_declaration (lowercase_identifier) @variable.other.member)
(struct_expression (labeled_expression (lowercase_identifier) @variable.other.member))
(struct_expression (labeled_expression_pun (lowercase_identifier) @variable.other.member))
(struct_field_expression (labeled_expression (lowercase_identifier) @variable.other.member))
(struct_field_expression (labeled_expression_pun (lowercase_identifier) @variable.other.member))
(struct_pattern (struct_field_pattern (labeled_pattern (lowercase_identifier) @variable.other.member)))
(struct_pattern (struct_field_pattern (labeled_pattern_pun (lowercase_identifier) @variable.other.member)))
(access_expression (accessor (dot_identifier) @variable.other.member))
(constructor_pattern_argument (lowercase_identifier) @variable.other.member "=")
(apply_expression (constructor_expression) (arguments (argument (labelled_argument (lowercase_identifier) @variable.other.member "="))))

; Attributes

(attribute) @attribute

; Function calls

(apply_expression (qualified_identifier (lowercase_identifier) @function))
(apply_expression (qualified_identifier (dot_lowercase_identifier) @function))
(package_apply_statement
  name: (package_statement_identifier) @function)

; Method calls

(method_expression (lowercase_identifier) @function.method)
(dot_apply_expression (dot_identifier) @function.method)
(dot_dot_apply_expression (dot_dot_identifier) @function.method)

; Function definitions

(function_definition (function_identifier (lowercase_identifier) @function))
(struct_constructor_declaration (lowercase_identifier) @function)
(function_alias_targets (lowercase_identifier) @function)
(function_alias_targets (dot_lowercase_identifier) @function)
(function_alias_target (lowercase_identifier) @function)
(trait_method_declaration (function_identifier) @function)
(impl_definition (function_identifier) @function)

; Method definitions

(function_definition (function_identifier (type_name (qualified_type_identifier)) (lowercase_identifier) @function.method))

; Labels

(loop_label) @label
("continue" (label) @label)
("break" (label) @label)
(package_argument
  label: (package_statement_identifier) @label)
(package_argument
  label: (string_literal) @label)
(package_map_entry
  key: (string_literal) @label)
(where_clause_field (lowercase_identifier) @label)

; Operators

[
  "+" "-" "*" "/" "%"
  "<<" ">>" "|" "&" "^"
  "=" "+=" "-=" "*=" "/=" "%="
  "<" ">" ">=" "<=" "==" "!="
  "&&" "||"
  "|>"
  "=>" "->"
  "!" "!!" "?"
] @operator

; Keywords

[ (mutability) "mut" ] @keyword.storage.modifier

[
  "struct" "enum" "type" "trait" "typealias" "traitalias" "suberror"
] @keyword.storage.type

[
  "pub" "priv" "readonly" "all" "open" "extern"
] @keyword.storage.modifier

[
  "guard" "let" "letrec" "and" "const"
  "with" "as" "is" "lexmatch?" "using" "where" "longest" "nobreak"
  "defer"
] @keyword

"derive" @keyword

[ "package" "import" ] @keyword.control.import

[ "fn" "test" "impl" "fnalias" ] @keyword.function
"return" @keyword.control.return
[ "while" "loop" "for" "break" "continue" "in" ] @keyword.control.repeat

[
  "if"
  "else"
  "match"
] @keyword.control.conditional

"async" @keyword

[ "try" "raise" "catch" "noraise" ] @keyword.control.exception

((lowercase_identifier) @keyword
 (#any-of? @keyword
           "import" "using"
           "proof_assert" "proof_let"
           "defer" "lexmatch" "recur"))

((lowercase_identifier) @keyword.control.exception
 (#eq? @keyword.control.exception "except"))

; Delimiters

[
  ";"
  ","
] @punctuation.delimiter

":" @punctuation.delimiter
"::" @punctuation.delimiter
"." @punctuation.delimiter
".." @punctuation.delimiter

(array_sub_pattern "..") @operator
(dot_dot_apply_expression (dot_dot_identifier ".." @punctuation.delimiter))

[
  "..<"
  "..="
  "..<="
  "..>"
  "..>="
] @operator

[
  "(" ")"
  "{" "}"
  "[" "]"
] @punctuation.bracket

; Literals

(string_interpolation) @string
(string_literal) @string
(multiline_string_literal) @string
(escape_sequence) @constant.character.escape

(interpolator
 "\\{" @punctuation.special
 "}" @punctuation.special)

(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(boolean_literal) @constant.builtin.boolean
(char_literal) @constant.character

; Comments

(comment) @comment
(block_comment) @comment.block

; Errors

(ERROR) @error
