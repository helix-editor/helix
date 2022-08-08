
; [
; (struct_definition)
; (component_definition)
; ] @indent

[
(field_declaration_list_body) 
(list_definition_body) 
(struct_field_declaration_list_body)
] @indent

; [
;   "{"
;   "}"
;   "("
;   ")"
;   (if_statement)
;   "else if"
;   "else"
; ] @branch

; "{" @indent
"}" @indent_end

(comment) @auto

(string) @auto

