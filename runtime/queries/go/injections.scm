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
    (function_declaration) ; `func`
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

; https://pkg.go.dev/fmt#Printf
; https://pkg.go.dev/fmt#Sprintf
; https://pkg.go.dev/fmt#Scanf
; https://pkg.go.dev/fmt#Errorf
((call_expression
  function: (selector_expression
    operand: (identifier) @_module
    field: (field_identifier) @_func)
  arguments: (argument_list
    . (interpreted_string_literal) @injection.content))
  (#eq? @_module "fmt")
  (#any-of? @_func "Printf" "Sprintf" "Scanf" "Errorf")
  (#set! injection.language "go-format-string"))

; https://pkg.go.dev/fmt#Fprintf
; https://pkg.go.dev/fmt#Fscanf
; https://pkg.go.dev/fmt#Sscanf
((call_expression
  function: (selector_expression
    operand: (identifier) @_module
    field: (field_identifier) @_func)
  arguments: (argument_list
    ; [(identifier) (interpreted_string_literal)]
    (_)
    ; (identifier)
    .
    (interpreted_string_literal) @injection.content))
  (#eq? @_module "fmt")
  (#any-of? @_func "Fprintf" "Fscanf" "Sscanf")
  (#set! injection.language "go-format-string"))

; https://pkg.go.dev/log#Printf
; https://pkg.go.dev/log#Fatalf
; https://pkg.go.dev/log#Panicf
; https://pkg.go.dev/log#Logger.Printf
; https://pkg.go.dev/log#Logger.Fatalf
; https://pkg.go.dev/log#Logger.Panicf
((call_expression
  function: (selector_expression
    operand: (identifier)
    field: (field_identifier) @_func)
  arguments: (argument_list
    . (interpreted_string_literal) @injection.content))
  (#any-of? @_func "Printf" "Fatalf" "Panicf")
  (#set! injection.language "go-format-string"))
