(comment) @comment.inside
(comment)+ @comment.around

(decl_def
  parameters: (parameter_bracks
    (parameter
      param_name: (identifier) @parameter.inside) @parameter.around)
  body: (block) @function.inside) @function.around

(record_entry
  value: (_) @entry.inside) @entry.around

(val_list
  item: (_) @entry.inside)

(val_table
  row: (val_list
    item: (_) @entry.inside) @entry.around)

(match_arm
  expression: (block) @entry.inside) @entry.around
