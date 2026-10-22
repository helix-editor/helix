(raw_statement) @markup.raw

(identifier) @variable
((identifier) @constant.builtin
  (#any-of? @constant.builtin "nil" "empty" "blank"))

(string) @string

(boolean) @constant.builtin.boolean

(number) @constant.numeric

(filter
  name: (identifier) @function)

[
  "{{"
  "}}"
  "{{-"
  "-}}"
  "{%"
  "%}"
  "{%-"
  "-%}"
] @punctuation.special

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

[
  ","
  "."
] @punctuation.delimiter

[
  "|"
  ":"
  "="
  ".."
  (predicate)
] @operator

[
  "and"
  "contains"
  "in"
  "or"
] @keyword.operator

[
  "as"
  "assign"
  "by"
  "capture"
  "cycle"
  "decrement"
  "echo"
  "endcapture"
  "endform"
  "endjavascript"
  "endpaginate"
  "endraw"
  "endschema"
  "endstyle"
  "endstylesheet"
  "form"
  "increment"
  "javascript"
  "liquid"
  "paginate"
  "raw"
  "schema"
  "style"
  "stylesheet"
  "with"
  (custom_unpaired_statement)
] @keyword

[
  "case"
  "else"
  "elsif"
  "endcase"
  "endif"
  "endunless"
  "if"
  "unless"
  "when"
] @keyword.control.conditional

[
  "endfor"
  "endtablerow"
  "for"
  "tablerow"
  (break_statement)
  (continue_statement)
] @keyword.control.repeat

[
  "include"
  "include_relative"
  "layout"
  "render"
  "section"
  "sections"
] @keyword.control.import

(comment) @comment

[
  (doc)
  (doc_content)
  (doc_example_annotation)
] @comment.block.documentation

[
  "doc"
  "enddoc"
  "@param"
  "@example"
  (doc_description_annotation)
] @keyword

(doc_type) @type

(doc_param_name) @variable.parameter
