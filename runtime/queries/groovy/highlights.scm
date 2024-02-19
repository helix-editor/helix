(unit
  (identifier) @variable)

(string
  (identifier) @variable)

(escape_sequence) @constant.character.escape

(block
  (unit
    (identifier) @namespace))

(func
  (identifier) @function)

(number) @constant.numeric

((identifier) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "true" "false"))

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z\\d_]*$"))

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

((identifier) @keyword.storage.modifier
  (#eq? @keyword.storage.modifier "static"))

((identifier) @keyword.storage.type
  (#any-of? @keyword.storage.type "class" "def" "interface"))

((identifier) @keyword
  (#any-of? @keyword
    "assert"
    "new"
    "extends"
    "implements"
    "instanceof"))

((identifier) @keyword.control.import
  (#any-of? @keyword.control.import "import" "package"))

((identifier) @keyword.storage.modifier
  (#any-of? @keyword.storage.modifier
    "abstract"
    "protected"
    "private"
    "public"))

((identifier) @keyword.control.exception
  (#any-of? @keyword.control.exception
    "throw"
    "finally"
    "try"
    "catch"))

(string) @string

[
  (line_comment)
  (block_comment)
] @comment

((block_comment) @comment.block.documentation
  (#match? @comment.block.documentation "^/[*][*][^*](?s:.)*[*]/$"))

((line_comment) @comment.block.documentation
  (#match? @comment.block.documentation "^///[^/]*.*$"))

[
  (operators)
  (leading_key)
] @operator

["(" ")" "[" "]" "{" "}"]  @punctuation.bracket
