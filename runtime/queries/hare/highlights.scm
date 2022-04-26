"abort" @keyword
"alloc" @keyword
"append" @keyword
"as" @keyword
"assert" @keyword
"bool" @keyword
"break" @keyword
"char" @keyword
"const" @keyword
"continue" @keyword
"def" @keyword
"defer" @keyword
"delete" @keyword
"else" @keyword
"enum" @keyword
"export" @keyword
"f32" @type
"f64" @type
"false" @constant
"fn" @keyword
"for" @keyword
"free" @keyword
"i16" @type
"i32" @type
"i64" @type
"i8" @type
"if" @keyword
"int" @type
"is" @keyword
"len" @keyword
"let" @keyword
"match" @keyword
"null" @constant
"nullable" @keyword
"offset" @keyword
"return" @keyword
"rune" @type
"size" @keyword
"static" @keyword
"str" @type
"struct" @keyword
"switch" @keyword
"true" @keyword
"type" @keyword
"u16" @type
"u32" @type
"u64" @type
"u8" @type
"uint" @type
"uintptr" @type
"union" @keyword
"use" @keyword
"void" @type
"..." @special 

"." @operator  
"!" @operator  
"~" @operator  
"?" @operator  
"*" @operator  
"/" @operator
"%" @operator  
"+" @operator  
"-" @operator 
"<<" @operator 
">>" @operator
"::" @operator 
"<" @operator  
"<=" @operator 
">" @operator  
">=" @operator
"==" @operator 
"!=" @operator 
"&" @operator  
"|" @operator  
"^" @operator  
"&&" @operator 
"||" @operator
"=" @operator     
"+=" @operator    
"-=" @operator   
"*=" @operator   
"/=" @operator   
"%=" @operator    
"&=" @operator    
"|=" @operator   
"<<=" @operator   
">>=" @operator 
"^=" @operator

":" @delimiter
";" @delimiter
"{" @delimiter
"}" @delimiter

(comment) @comment

(string_constant) @string
(escape_sequence) @type
(rune_constant) @string
(integer_constant) @number 
(floating_constant) @number

(call_expression
  (postfix_expression) @function)

(function_declaration
  name: (identifier) @function)

(parameter (name) @variable.parameter)

(field_access_expression
  selector: (name) @field)
(decl_attr) @special
(fndec_attrs) @special

(identifier) @variable

