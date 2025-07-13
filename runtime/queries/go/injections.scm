((comment) @injection.content
 (#set! injection.language "comment"))

; Inject markdown into documentation comments
;
; Go's comments are documentation comments when they are directly followed
; by one of Go's statements (e.g. `type`, `func`, `const`)
;
; This is only a partial implementation, which covers only
; block comments. For line comments (which are more common),
; upstream changes to the grammar are required.
(
  (comment) @injection.content . (comment)* . [
    (package_clause) ; `package`
    (type_declaration) ; `type`
    (method_declaration) ; `func`
    (var_declaration) ; `var`
    (const_declaration) ; `const`
    ; var (
    ; 	A = 1
    ; 	B = 2
    ; )
    (const_spec)
    ; const (
    ; 	A = 1
    ; 	B = 2
    ; )
    (var_spec)
  ]
  (#set! injection.language "markdown"))

(call_expression
  (selector_expression) @_function
  (#any-of? @_function "regexp.Match" "regexp.MatchReader" "regexp.MatchString" "regexp.Compile" "regexp.CompilePOSIX" "regexp.MustCompile" "regexp.MustCompilePOSIX")
  (argument_list
    .
    [
      (raw_string_literal)
      (interpreted_string_literal)
    ] @injection.content
    (#set! injection.language "regex")))
