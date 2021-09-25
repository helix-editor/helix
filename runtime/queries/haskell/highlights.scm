(variable) @variable
(operator) @operator
(exp_name (constructor) @constructor)
(constructor_operator) @operator
(module) @namespace
(type) @type
(type) @class
(constructor) @constructor
(pragma) @pragma
(comment) @comment
(signature name: (variable) @fun_type_name)
(function name: (variable) @function)
(constraint class: (class_name (type)) @class)
(class (class_head class: (class_name (type)) @class))
(instance (instance_head class: (class_name (type)) @class))
(integer) @number
(exp_literal (float)) @number
(char) @literal
(con_unit) @literal
(con_list) @literal
(tycon_arrow) @operator
(where) @keyword
"module" @keyword
"let" @keyword
"in" @keyword
"class" @keyword
"instance" @keyword
"data" @keyword
"newtype" @keyword
"family" @keyword
"type" @keyword
"import" @keyword
"qualified" @keyword
"as" @keyword
"deriving" @keyword
"via" @keyword
"stock" @keyword
"anyclass" @keyword
"do" @keyword
"mdo" @keyword
"rec" @keyword
[
  "("
  ")"
] @punctuation.bracket
