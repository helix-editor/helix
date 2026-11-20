(comment) @comment

(pass_type) @keyword

(nil) @constant.builtin

(number) @constant.numeric.integer

(pass
  file: (name) @function)

(inactive) @comment

; An inactive pass reads as switched off.
(pass
  inactive: (inactive)
  type: (pass_type) @comment)
