(let
  pattern: (call)
  value: (_) @function.inside) @function.around

(call
  (group
    ((_) @parameter.inside . ","? @parameter.around) @parameter.around))

(lambda
  pattern: 
    (group
      ((_) @parameter.inside . ","? @parameter.around) @parameter.around)
  value: (_) @function.inside) @function.around

(group
  [
    (tagged (_) @entry.inside)
    (_)
  ] @entry.around)

(comment) @comment.inside

(comment)+ @comment.around
