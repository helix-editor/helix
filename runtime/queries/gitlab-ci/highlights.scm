(boolean_scalar) @constant.builtin.boolean
(null_scalar) @constant.builtin
(double_quote_scalar) @string
(single_quote_scalar) @string
(block_scalar) @string
(string_scalar) @string
(escape_sequence) @constant.character.escape
(integer_scalar) @constant.numeric.integer
(float_scalar) @constant.numeric.float
(comment) @comment
(anchor_name) @type
(alias_name) @type
(tag) @type
(yaml_directive) @keyword

(block_mapping_pair
  key: (flow_node [(double_quote_scalar) (single_quote_scalar)] @variable.other.member))
(block_mapping_pair
  key: (flow_node (plain_scalar (string_scalar) @variable.other.member)))

(flow_mapping
  (_ key: (flow_node [(double_quote_scalar) (single_quote_scalar)] @variable.other.member)))
(flow_mapping
  (_ key: (flow_node (plain_scalar (string_scalar) @variable.other.member))))

[
","
"-"
":"
">"
"?"
"|"
] @punctuation.delimiter

[
"["
"]"
"{"
"}"
] @punctuation.bracket

["*" "&" "---" "..."] @punctuation.special


; Highlight the toplevel keys differently as keywords
(block_mapping_pair
  key: (flow_node (plain_scalar (string_scalar) @keyword (#any-of? @keyword "variables" "stages" "default" "include" "workflow"))) )

; Highlight the builtin stages differently
; <https://docs.gitlab.com/ci/yaml/#stages>
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @variable.other.member (#eq? @variable.other.member "stage")))
  value: (flow_node
           (plain_scalar
             (string_scalar) @constant.builtin (#any-of? @constant.builtin ".pre" "build" "test" "deploy" ".post"))))
; e.g.
; ```
; stages:
;   - build
;   - test
; ```
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @keyword (#eq? @keyword "stages")))
  value: (block_node
           (block_sequence
             (block_sequence_item
               (flow_node
                 (plain_scalar
                   (string_scalar) @constant.builtin (#any-of? @constant.builtin ".pre" "build" "test" "deploy" ".post")))))))


; Highlight defined variable names as @variable
; Matches on:
; ```
; variables:
;   <variable>: ...
; ```
(block_mapping_pair
  key: (flow_node
         (plain_scalar
           (string_scalar) @keyword (#eq? @keyword "variables")))
  value: (block_node
           (block_mapping
             (block_mapping_pair
               key: (flow_node) @variable)+)))
