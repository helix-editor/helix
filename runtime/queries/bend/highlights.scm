; General captures precede more specific contextual captures.
(identifier) @variable
(comment) @comment
(string) @string
(character) @character
(escape_sequence) @string.escape
[(integer) (natural)] @number
(float) @number.float
(quantity) @constant.builtin
(kind) @type.builtin
(decorator) @attribute
(hole) @constant.builtin
(module_path) @string.special.path

(function_definition name: (identifier) @function)
(law_declaration name: (identifier) @function)
(type_declaration name: (identifier) @type)
(constructor_declaration name: (identifier) @constructor)
(constructor_expression name: (identifier) @constructor)
(type_application name: (identifier) @type)
(call_expression function: (identifier) @function.call)
(parameter name: (identifier) @variable.parameter)
(quantifier name: (identifier) @variable.parameter)
(dependent_type name: (identifier) @variable.parameter)
(lambda_expression parameter: (identifier) @variable.parameter)
(lambda_expression parameter: (reusable_expression (identifier) @variable.parameter))
(import_declaration alias: (identifier) @module)
(do_expression monad: (identifier) @type)

((identifier) @type.builtin
  (#any-of? @type.builtin
    "Nat" "U32" "F32" "Char" "String" "Bool" "Unit" "Empty" "IO"))

["def" "law" "type" "is" "for" "exs" "where" "do"] @keyword
["match" "case"] @keyword.conditional
["import" "as"] @keyword.import
"return" @keyword.return
"Base" @module

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["," ":" ";"] @punctuation.delimiter
["=" "->" "=>" "<-" "+" "-" "*" "/" "%" "^" "&" "|"
 "<&>" "<>" "++" "&&" "||" ".&." ".|." ".^." "<<" ">>"
 "<" ">" "<=" ">=" "==" "!=" "@" "~" "!" "?" "\\"] @operator
