; function definitions
(list
  .
  (symbol) @_define
  .
  (list
    .
    [
      (symbol) @name
      ; for curried functions
      (list . (symbol) @name)
      (list . (list . [(symbol) (list)] @name))
    ])
  (#eq? @_define "define")) @definition.function

(list
  .
  (symbol) @_define
  .
  (symbol) @name
  (#eq? @_define "define")) @definition.constant
