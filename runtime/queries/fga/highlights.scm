; Expressions 

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (identifier) @function.method))


; Type Definitions

(type_declaration (identifier) @type)

(definition 
  relation: (identifier) @variable)


; Relation Definitions 

(relation_def (identifier) @variable.other.member)

(direct_relationship (identifier) @type)
(direct_relationship (conditional (identifier) @function))

(relation_ref 
  . (identifier) @type
  (identifier) @variable.other.member)

(indirect_relation
  . (identifier) @variable.other.member
    (identifier) @variable)


; Condition Defintions

(condition_declaration
  name: (identifier) @function)

(condition_declaration (param (identifier) @variable.parameter))

(binary_expression (identifier) @variable)

((type_identifier) @type.builtin
  (#any-of? @type.builtin "string" "int" "map" "uint" "list" "timestamp" "bool" "duration" "double" "ipaddress"))


; Operators

[
  "!="
  "%"
  "&"
  "&&"
  "&^"
  "*"
  "+"
  "-"
  "/"
  "<"
  "<<"
  "<="
  "=="
  ">"
  ">="
  ">>"
  "^"
  "|"
  "||"
] @operator

[
  "or"
  "and"
  "but not"
  "from"
  "with"
] @keyword.operator

; Keywords

[
  "model"
  "schema"
  "type"
  "relations"
  "define"
] @keyword

[
  "condition"
] @keyword.function

; Misc

(version) @constant.numeric
(comment) @comment
