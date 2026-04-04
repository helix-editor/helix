(comment) @comment.inside

(comment)+ @comment.around

(function
  body: (_) @function.inside) @function.around

(args
  ((arg) @parameter.inside . ","? @parameter.around) @parameter.around)

(call_args
  ((call_arg) @parameter.inside . ","? @parameter.around) @parameter.around)

(map
  ((entry_inline) @entry.inside . ","? @entry.around) @entry.around)

(map_block
  ((entry_block) @entry.inside) @entry.around)

(list
  ((element) @entry.inside . ","? @entry.around) @entry.around)

(tuple
  (_) @entry.around)

(assign
  (meta (test))
  (function body: (_) @test.inside)
) @test.around

(entry_block
  key: (meta (test))
  value: (function body: (_) @test.inside)
) @test.around
