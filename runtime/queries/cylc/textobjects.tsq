(comment) @comment.inside

(comment)+ @comment.around

(_
  brackets_open: _
  name: _?
  brackets_close: _
  _* @class.inside) @class.around

(setting
  value: _? @function.inside) @function.around

(graph_setting
  value: _? @function.inside) @function.around

(graph_string_content
  (graph_task) @entry.inside)

(task_parameter
  ((_) @parameter.inside
    .
    ","? @parameter.around) @parameter.around)
