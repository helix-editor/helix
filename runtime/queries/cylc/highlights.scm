[
  (jinja2_expression)
  (jinja2_statement)
  (jinja2_comment)
  (jinja2_shebang)
] @special

(include_statement
  directive: _ @keyword.directive
  path: _ @string.special.path)

(comment) @comment.line

(graph_section
  name: _? @label)

(task_section
  name: (_
    (task_name) @namespace))

(top_section
  brackets_open: _ @punctuation.bracket
  name: _? @label
  brackets_close: _ @punctuation.bracket)

(sub_section_1
  brackets_open: _ @punctuation.bracket
  name: _? @label
  brackets_close: _ @punctuation.bracket)

(sub_section_2
  brackets_open: _ @punctuation.bracket
  name: _? @label
  brackets_close: _ @punctuation.bracket)

(runtime_section
  brackets_open: _ @punctuation.bracket
  name: _? @label
  brackets_close: _ @punctuation.bracket)

(graph_setting
  key: (_) @constant.numeric.integer
  operator: (_)? @operator)

(quoted_graph_string
  quotes_open: _ @string
  quotes_close: _ @string)

(multiline_graph_string
  quotes_open: _ @string
  quotes_close: _ @string)

[
  (graph_logical)
  (graph_arrow)
  (graph_parenthesis)
] @operator

(intercycle_annotation
  (recurrence) @constant.numeric.integer)

(graph_task
  xtrigger: _? @operator
  suicide: _? @operator
  name: _ @namespace)

(task_parameter
  "<" @tag
  name: (_)? @special
  ","? @tag
  "="? @tag
  selection: (_)? @special
  ">" @tag)

(intercycle_annotation
  "[" @tag
  (recurrence)? @constant.numeric.integer
  "]" @tag)

(task_output
  ":" @tag
  (nametag) @variable.other)

(task_output
  "?"? @tag)

(setting
  key: (key) @variable
  operator: (_)? @operator
  value: [
    (unquoted_string) @string
    (quoted_string) @string
    (multiline_string) @string
    (boolean) @constant.builtin.boolean
    (integer) @constant.numeric.integer
  ]?)

(datetime) @constant.numeric.float
