((text) @injection.content
 (#set! injection.language "html")
 (#set! injection.combined))

((comment) @injection.content
 (#set! injection.language "comment"))

((function_call_expression
 function: (name) @_function
 arguments: (arguments . (argument (_ (string_value) @injection.content))))
 (#match? @_function "^preg_")
 (#set! injection.language "regex"))

((function_call_expression
 function: (name) @_function
 arguments: (arguments (_) (argument (_ (string_value) @injection.content))))
 (#match? @_function "^mysqli_")
 (#set! injection.language "sql"))

((member_call_expression
 object: (_)
 name: (name) @_function
 arguments: (arguments . (argument (_ (string_value) @injection.content))))
 (#match? @_function "^(prepare|query)$")
 (#set! injection.language "sql"))
