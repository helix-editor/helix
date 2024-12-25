;; Queries for helix to select textobjects: https://docs.helix-editor.com/usage.html#textobjects
;;  function.inside
;; function.around
;; class.inside
;; class.around
;; test.inside
;; test.around
;; parameter.inside
;; comment.inside
;; comment.around

;; Queries for navigating using textobjects

[
  (line_comment)
  (block_comment)
] @comment.inside

(line_comment)+ @comment.around
(block_comment) @comment.around

(entity members: (members)? @class.inside) @class.around
(object members: (members)? @class.inside) @class.around

(method
  body: (block)? @function.inside
) @function.around
(behavior
  body: (block)? @function.inside
) @function.around
(constructor
  body: (block)? @function.inside
) @function.around
(lambda
  body: (block)? @function.inside
) @function.outside

(params
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around
)
(lambda
  params: ((_) @parameter.inside . ","? @parameter.around) @parameter.around
)
(typeargs
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around
)
(typeparams
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around
)
(arguments
  positional: (positional_args
                ((_) @parameter.inside . ","? @parameter.around)? @parameter.around)
  ; TODO: get named args right
  named: (named_args ((_) @parameter.inside . ","? @parameter.around)? @parameter.around)
)

(
  (entity
    provides: (type (nominal_type name: (identifier) @_provides))
    members: (members) @test.inside
  ) @test.outside
  (#eq? @_provides "UnitTest")
)

