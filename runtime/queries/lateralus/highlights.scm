; ---------------------------------------------------------------------------
; tree-sitter-lateralus highlights query
; Used by: nvim-treesitter, Helix, Zed, Neovim, …
; ---------------------------------------------------------------------------

; Comments
(line_comment) @comment
(block_comment) @comment

; Imports
"import" @keyword.import
"use"    @keyword.import

(import_decl (path (identifier) @namespace))

; Keywords — control flow
"if"     @keyword.conditional
"else"   @keyword.conditional
"match"  @keyword.conditional
"while"  @keyword.repeat
"for"    @keyword.repeat
"in"     @keyword.repeat
"return" @keyword.return

; Keywords — declarations
"fn"     @keyword.function
"struct" @keyword.type
"enum"   @keyword.type
"impl"   @keyword.type
"type"   @keyword.type
"const"  @keyword
"let"    @keyword
"mut"    @keyword.modifier

; Visibility
(visibility) @keyword.modifier

; Primitive types
(primitive_type) @type.builtin

; Type names (CamelCase identifiers in type position)
(struct_decl name: (identifier) @type)
(enum_decl   name: (identifier) @type)
(type_alias  name: (identifier) @type)
(impl_block  target: (identifier) @type)

; Functions
(function_decl name: (identifier) @function)
(call_expression (identifier) @function.call)

; Fields
(field_decl name: (identifier) @property)
(field_access (identifier) @property)

; Parameters
(parameter name: (identifier) @variable.parameter)

; Variables
(let_decl   name: (identifier) @variable)
(const_decl name: (identifier) @constant)

; Literals
(string_literal)  @string
(number_literal)  @number
(boolean_literal) @boolean
(null_literal)    @constant.builtin

; Operators
"|>" @operator
"->" @operator
"=>" @operator
"::" @operator
"+"  @operator
"-"  @operator
"*"  @operator
"/"  @operator
"%"  @operator
"==" @operator
"!=" @operator
"<"  @operator
">"  @operator
"<=" @operator
">=" @operator
"&&" @operator
"||" @operator
"="  @operator

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"<" @punctuation.bracket
">" @punctuation.bracket
"," @punctuation.delimiter
";" @punctuation.delimiter
":" @punctuation.delimiter
"." @punctuation.delimiter

; Identifiers — generic fallback
(identifier) @variable
