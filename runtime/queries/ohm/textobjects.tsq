; See: https://docs.helix-editor.com/guides/textobject.html

; function.inside & around
; ------------------------

(rule
  body: (_) @function.inside) @function.around

; class.inside & around
; ---------------------

(grammar
  body: (_) @class.inside) @class.around

; parameter.inside & around
; -------------------------

(formals
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(rule_body
  ((_) @parameter.inside . "|"? @parameter.around) @parameter.around)

(params
  ((_) @parameter.inside . ","? @parameter.around) @parameter.around)

(alt
  ((_) @parameter.inside . "|"? @parameter.around) @parameter.around)

; comment.inside
; --------------

(multiline_comment)+ @comment.inside
(singleline_comment)+ @comment.inside

; comment.around
; --------------

(multiline_comment)+ @comment.around
(singleline_comment)+ @comment.around
