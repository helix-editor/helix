; Class
(class) @class.around

(class [(constant) (scope_resolution)] !superclass
  (_)+ @class.inside) 

(class [(constant) (scope_resolution)] (superclass)
  (_)+ @class.inside) 

(singleton_class
  value: (_)
  (_)+ @class.inside) @class.around

(call
  receiver: (constant) @class_const
  method: (identifier) @class_method
  (#match? @class_const "Class")
  (#match? @class_method "new")
  (do_block (_)+ @class.inside)) @class.around

; Functions
(method) @function.around

(method (identifier) (method_parameters)
  (_)+ @function.inside)
  
(do_block !parameters
  (_)+ @function.inside)
  
(do_block (block_parameters)
  (_)+ @function.inside)
    
(block (block_parameters)
  (_)+ @function.inside)
      
(block !parameters
  (_)+ @function.inside)
      
(method (identifier) !parameters
  (_)+  @function.inside)

; Parameters      
(method_parameters
  (_) @parameter.inside)
        
(block_parameters 
  (_) @parameter.inside)
        
(lambda_parameters 
  (_) @parameter.inside)

; Comments
(comment) @comment.inside 
(comment)+ @comment.around
