; Brim highlighting. Capture names follow the common tree-sitter
; conventions (Neovim, Helix, Zed).

(comment) @comment

(integer) @constant.numeric.integer
(string) @string
(escape_sequence) @constant.character.escape
(boolean) @constant.builtin.boolean
(self) @variable.builtin

(primitive_type) @type.builtin
(type_identifier) @type
(path_type (identifier) @type)
((path_type (identifier) @type.builtin)
  (#any-of? @type.builtin "Option" "Result" "List" "Map"))
(type_parameters (identifier) @type)
(scoped_type_identifier (identifier) @namespace)

(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
((call_expression function: (identifier) @function.builtin)
  (#any-of? @function.builtin
    "print" "eprint" "to_string" "read_line" "read_file" "write_file" "args" "env" "exit"
    "assert" "assert_eq"))
((call_expression function: (identifier) @constructor)
  (#any-of? @constructor "Some" "Ok" "Err"))
((identifier) @constant.builtin
  (#eq? @constant.builtin "None"))
(method_call_expression method: (identifier) @function.method)

(parameter name: (identifier) @variable.parameter)
(field_declaration name: (field_identifier) @variable.other.member)
(field_initializer name: (field_identifier) @variable.other.member)
(field_expression field: (field_identifier) @variable.other.member)
(variant name: (identifier) @constructor)
(variant_pattern name: (identifier) @constructor)
(module_path) @namespace

[
  "fn" "struct" "enum" "impl" "let"
] @keyword

[
  "if" "else" "match"
] @keyword.control.conditional

[
  "while" "for" "in"
] @keyword.control.repeat

(break_statement) @keyword.control.repeat
(continue_statement) @keyword.control.repeat
"return" @keyword.control.return

[
  "import" "as"
] @keyword.control.import

(visibility) @keyword.storage.modifier
"mut" @keyword.storage.modifier

[
  "=" "+=" "-=" "*=" "/=" "%=" "+" "-" "*" "/" "%" "==" "!=" "<" "<=" ">" ">="
  "&&" "||" "!" "?" ".." "->" "=>"
] @operator

[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket
[ "," ";" ":" "." ] @punctuation.delimiter
