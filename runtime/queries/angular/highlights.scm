; inherits: html

; --- Identifiers & Variables ---

; Any bare identifier not matched by a more specific rule: foo, myVar
(identifier) @variable

; --- Operators ---

; The | pipe operator between expression and pipe call: value | pipe
(pipe_operator) @operator

; --- Literals ---

; Numeric literals: 42, 3.14
(number) @constant.numeric

; --- Functions & Pipes ---

; Pipe name: the "date" in value | date:'short'
(pipe_call
  name: (identifier) @function)

; Pipe argument identifiers: the "fmt" in value | date:fmt
(pipe_call
  arguments: (pipe_arguments
    (identifier) @variable.parameter))

; --- Loop Variables ---

; @for loop variable: the "item" in @for (item of items; track item.id)
(for_declaration
  name: (identifier) @variable.parameter)

; Single-param arrow function parameter: the "x" in x => x * 2
(arrow_function
  parameters: (identifier) @variable.parameter)

; Multi-param arrow function parameters: the "x", "y" in (x, y) => x + y
(arrow_function_parameters
  (identifier) @variable.parameter)

; --- Style & Units ---

; CSS property unit suffix in style bindings: the "px" in [style.width.px]
(style_unit) @variable

; Time/size units in @defer timed expressions: the "ms" in on timer(500ms)
(unit) @constant

; --- Strings ---

; String literals in template expressions: 'hello', "world"
(string) @string

; Regular expression pattern body: the /pattern/ part
(regular_expression
  pattern: (regular_expression_pattern) @string.regexp)

; Regular expression flags: the "gi" in /pattern/gi
(regular_expression
  flags: (regular_expression_flags) @string.special)

; --- Class Bindings ---

; CSS class name in [class.active]: the "active" part
(class_name) @tag.attribute

; Bound identifier in [class.active]="expr": the "active" identifier node
(class_binding
  (identifier) @variable.other.member)

; --- Template Reference & Structural Aliases ---

; @if as-alias identifier: the "myVal" in @if (expr; as myVal)
(if_reference
  (identifier) @variable)

; *ngIf as-alias: the "aliasName" in *ngIf="expr as aliasName"
(structural_expression
  alias: (identifier) @variable)

; *ngFor let-variable in structural expression: the "i" in let i = index
(structural_expression
  named: (identifier) @variable.parameter)

; Legacy structural directive asterisk and name: the "*" and "ngFor" in *ngFor
(structural_directive
  "*" @keyword
  (identifier) @keyword)

; Template reference variable attribute: the "#hero" in <div #hero>
(attribute
  (attribute_name) @label
  (#match? @label "^#"))

; --- Binding Names ---

; Property/event/two-way binding target identifier: the "ngModel" in [(ngModel)]
(binding_name
  (identifier) @keyword.control.directive)

; Event binding target: the "click" in (click)="handler()"
(event_binding
  (binding_name
    (identifier) @keyword.control))

; Quotes wrapping event binding value: the '"' in (click)="handler()"
(event_binding
  "\"" @punctuation.delimiter)

; Quotes wrapping property binding value: the '"' in [prop]="val" or [prop]=""
(property_binding
  [
    "\""
    "\"\""
  ] @punctuation.delimiter)

; Structural assignment operator keyword: the "of" in *ngFor="let x of items"
(structural_assignment
  operator: (identifier) @keyword.control.directive)

; --- Member Access ---

; Object property access: the "name" in user.name or user?.name
(member_expression
  property: (identifier) @variable.other.member)

; --- Call Expressions ---

; Function call identifier: the "myFn" in myFn()
(call_expression
  function: (identifier) @function)

; Angular's $any() type-cast escape hatch
(call_expression
  function: ((identifier) @function.builtin
    (#eq? @function.builtin "$any")))

; $implicit key in object literals used by structural directives
(pair
  key: ((identifier) @variable.builtin
    (#eq? @variable.builtin "$implicit")))

; --- Control Flow Keywords ---

; All control flow keywords not matched below: @let, @defer, @placeholder, @loading
(control_keyword) @keyword.control

; Special keywords inside control flow: "of" in @for, "as" in @if, "track" etc.
(special_keyword) @keyword.control

; "prefetch" modifier in @defer (prefetch on interaction)
(prefetch_keyword) @keyword.control

; Loop keywords: @for, @empty
((control_keyword) @keyword.control.repeat
  (#any-of? @keyword.control.repeat "for" "empty"))

; Conditional keywords: @if, @else if, @else, @switch, @case, @default
((control_keyword) @keyword.control.conditional
  (#any-of? @keyword.control.conditional "if" "else" "switch" "case" "default"))

; Deferred loading keywords: @defer, @placeholder, @loading
((control_keyword) @keyword.control
  (#any-of? @keyword.control "defer" "placeholder" "loading"))

; Error block keyword: @error inside @defer
((control_keyword) @keyword.control.exception
  (#eq? @keyword.control.exception "error"))

; --- Built-in Constants & Variables ---

; Boolean literals: true, false
((identifier) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "true" "false"))

; Built-in template variables: this, $event
((identifier) @variable.builtin
  (#any-of? @variable.builtin "this" "$event"))

; Null literal: null
((identifier) @constant.builtin
  (#eq? @constant.builtin "null"))

; --- Operators ---

; Ternary operator token: the "?" and ":" in cond ? a : b
(ternary_operator) @operator

; Conditional (ternary) operator node
(conditional_operator) @operator

; Unary operator: !, -, + in !flag, -n, +n
(unary_operator) @operator

; --- Punctuation: Brackets ---

; Standard expression brackets: ( ) [ ] { } and @ sigil
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "@"
] @punctuation.bracket

; Two-way binding delimiters: [( and )] in [(ngModel)]="val"
(two_way_binding
  [
    "[("
    ")]"
  ] @punctuation.bracket)

; Interpolation delimiters: {{ and }} in {{ expr }}
[
  "{{"
  "}}"
] @punctuation.special

; Template literal interpolation delimiters: ${ and } in `text ${expr}`
(template_substitution
  [
    "${"
    "}"
  ] @punctuation.special)

; --- Strings (continued) ---

; Raw text characters inside template strings between substitutions
(template_chars) @string

; --- Punctuation: Delimiters ---

; Statement/property separators and accessor operators: ; . , ?. !.
[
  ";"
  "."
  ","
  "?."
  "!."
] @punctuation.delimiter

; Arrow in arrow function expression: => in x => x + 1
(arrow_function
  "=>" @operator)

; Spread operator in object literals: ...obj in { ...defaults, key: val }
(object
  (spread
    "..." @operator))

; Spread operator in array literals: ...arr in [1, ...rest]
(array
  (spread
    "..." @operator))

; Spread operator in function arguments: ...args in fn(...args)
(arguments
  (spread
    "..." @operator))

; Nullish coalescing operator: ?? in expr ?? fallback
(nullish_coalescing_expression
  (coalescing_operator) @operator)

; String concatenation plus operator: + in 'Hello ' + name
(concatenation_expression
  "+" @operator)

; --- ICU (Internationalization) ---

; ICU message format clause type: "plural", "select" in {count, plural, ...}
(icu_clause) @keyword.control.directive

; ICU category label: "one", "other", "=0" in plural/select expressions
(icu_category) @keyword.control.conditional

; --- Binary Operators ---

; All binary infix operators in expressions
(binary_expression
  [
      "-"
      "&&"
      "+"
      "<"
      "<="
      "="
      "=="
      "==="
      "!="
      "!=="
      ">"
      ">="
      "*"
      "/"
      "||"
      "%"
    ] @operator)

; --- HTML ---

; HTML/component element tag name: the "div" in <div> or <app-root>
(tag_name) @tag
