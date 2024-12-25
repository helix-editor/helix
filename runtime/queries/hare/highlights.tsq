(type) @type
(type "const" @type)

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

"return" @keyword.control.return

[
  "abort"
  "assert"
] @keyword.control.exception

"fn" @keyword.function

[
  "alloc"
  "append"
  "as"
  "bool"
  "case"
  "const"
  "def"
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
  "struct"
  "type"
  "union"
  "yield"
] @keyword

"static" @keyword.storage.modifier

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
(literal "void") @constant.builtin

(string_literal) @string
(escape_sequence) @constant.character.escape
(rune_literal) @string
(integer_literal) @constant.numeric.integer
(floating_literal) @constant.numeric.float

(call_expression
  (postfix_expression) @function)
(size_expression "size" @function.builtin)

(function_declaration
  name: (identifier) @function)

(parameter (name) @variable.parameter)

(field_access_expression
  selector: (name) @variable.other.member)
(decl_attr) @special
(fndec_attrs) @special

(identifier) @variable
(struct_union_field (name)) @variable
