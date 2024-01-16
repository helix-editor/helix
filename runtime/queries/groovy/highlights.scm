(unit
  (identifier) @variable)
(string
  (identifier) @variable)

(escape_sequence) @string.escape

(block
  (unit
    (identifier) @namespace))

(func
  (identifier) @function)

(number) @number

((identifier) @boolean
  (#any-of? @boolean "true" "false" "True" "False"))

((identifier) @constant
  (#lua-match? @constant "^[A-Z][A-Z%d_]*$"))

((identifier) @constant.builtin
  (#eq? @constant.builtin "null"))

((identifier) @type
  (#any-of? @type
    "String"
    "Map"
    "Object"
    "Boolean"
    "Integer"
    "List"))

((identifier) @function.builtin
  (#any-of? @function.builtin
    "void"
    "id"
    "version"
    "apply"
    "implementation"
    "testImplementation"
    "androidTestImplementation"
    "debugImplementation"))

((identifier) @keyword
  (#any-of? @keyword
    "static"
    "class"
    "def"
    "import"
    "package"
    "assert"
    "extends"
    "implements"
    "instanceof"
    "interface"
    "new"))

((identifier) @type.qualifier
  (#any-of? @type.qualifier
    "abstract"
    "protected"
    "private"
    "public"))

((identifier) @exception
  (#any-of? @exception
    "throw"
    "finally"
    "try"
    "catch"))

(string) @string

[
  (line_comment)
  (block_comment)
] @comment @spell

((block_comment) @comment.documentation
  (#lua-match? @comment.documentation "^/[*][*][^*].*[*]/$"))

((line_comment) @comment.documentation
  (#lua-match? @comment.documentation "^///[^/]"))

((line_comment) @comment.documentation
  (#lua-match? @comment.documentation "^///$"))

[
  (operators)
  (leading_key)
] @operator

["(" ")" "[" "]" "{" "}"]  @punctuation.bracket
