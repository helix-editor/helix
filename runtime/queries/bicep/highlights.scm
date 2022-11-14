; Keywords

[
  "module"
  "var"
  "param"
  "import"
  "resource"
  "existing"
  "if"
  "targetScope"
  "output"
] @keyword

; Functions

(decorator) @function.builtin

(functionCall) @function

(functionCall
  (functionArgument
    (variableAccess) @variable))

; Literals/Types

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(resourceDeclaration
  (string
    (stringLiteral) @string.special))

(moduleDeclaration
  (string
    (stringLiteral) @string.special))

[
  (string)
  (stringLiteral)
] @string

(nullLiteral) @keyword
(booleanLiteral) @constant.builtin.boolean
(integerLiteral) @constant.numeric.integer
(comment) @comment

(string
  (variableAccess
    (identifier) @variable))

(type) @type

; Variables

(localVariable) @variable

; Statements

(object
  (objectProperty
    (identifier) @identifier))

(propertyAccess
  (identifier) @identifier)
  
(ifCondition) @keyword.control.conditional
