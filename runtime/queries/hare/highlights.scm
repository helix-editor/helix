[
  "f32"
  "f64"
  "i16"
  "i32"
  "i64"
  "i8"
  "int"
  "rune"
  "str"
  "u16"
  "u32"
  "u64"
  "u8"
  "uint"
  "uintptr"
  "void"
] @type


[
  "else"
  "if"
  "match"
  "switch"
] @keyword.control.conditional

[
  "export"
  "use"
] @keyword.control.import

[
  "continue"
  "for"
  "break"
] @keyword.control.repeat

[
  "return"
  "yield"
] @keyword.control.return

[
  "abort"
  "assert"
] @keyword.control.exception

[
  "def"
  "fn"
] @keyword.function

[
  "alloc"
  "append"
  "as"
  "bool"
  "char"
  "const"
  "defer"
  "delete"
  "enum"
  "free"
  "is"
  "len"
  "let"
  "match"
  "nullable"
  "offset"
  "size"
  "static"
  "struct"
  "type"
  "union"
] @keyword

[
  "."  
  "!"  
  "~"  
  "?"  
  "*"  
  "/"
  "%"  
  "+"  
  "-" 
  "<<" 
  ">>"
  "::" 
  "<"  
  "<=" 
  ">"  
  ">="
  "==" 
  "!=" 
  "&"  
  "|"  
  "^"  
  "&&" 
  "||"
  "="     
  "+="    
  "-="   
  "*="   
  "/="   
  "%="    
  "&="    
  "|="   
  "<<="   
  ">>=" 
  "^="
  "=>"
] @operator

[
  "("
  ")"
  "["
  "]"
  ")"
  "{"
  "}"
] @punctuation.bracket

[
  ":"
  ";"
] @punctuation.delimiter

"..." @special 

(comment) @comment

[
  "false"
  "null"
  "true"
] @constant.builtin

(string_constant) @string
(escape_sequence) @constant.character.escape
(rune_constant) @string
(integer_constant) @constant.numeric.integer 
(floating_constant) @constant.numeric.float

(call_expression
  (postfix_expression) @function)

(function_declaration
  name: (identifier) @function)

(parameter (name) @variable.parameter)

(field_access_expression
  selector: (name) @variable.other.member)
(decl_attr) @special
(fndec_attrs) @special

(identifier) @variable

