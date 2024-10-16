; inherits: python

; Compound directives
[
  "rule"
  "checkpoint"
  "module"
] @keyword

; Top level directives (eg. configfile, include)
(module
  (directive
    name: _ @keyword))

; Subordinate directives (eg. input, output)
((_)
  body: (_
    (directive
      name: _ @label)))

; rule/module/checkpoint names
(rule_definition
  name: (identifier) @type)

(module_definition
  name: (identifier) @type)

(checkpoint_definition
  name: (identifier) @type)

; Rule imports
(rule_import
  "use" @keyword.import
  "rule" @keyword.import
  "from" @keyword.import
  "exclude"? @keyword.import
  "as"? @keyword.import
  "with"? @keyword.import)

; Rule inheritance
(rule_inheritance
  "use" @keyword
  "rule" @keyword
  "with" @keyword)

; Wildcard names
(wildcard (identifier) @variable)
(wildcard (flag) @variable.parameter.builtin)

; builtin variables
((identifier) @variable.builtin
  (#any-of? @variable.builtin "checkpoints" "config" "gather" "rules" "scatter" "workflow"))

; References to directive labels in wildcard interpolations
; the #any-of? queries are moved above the #has-ancestor? queries to
; short-circuit the potentially expensive tree traversal, if possible
; see:
; https://github.com/nvim-treesitter/nvim-treesitter/pull/4302#issuecomment-1685789790
; directive labels in wildcard context
((wildcard
  (identifier) @label)
  (#any-of? @label "input" "log" "output" "params" "resources" "threads" "wildcards"))

((wildcard
  (attribute
    object: (identifier) @label))
  (#any-of? @label "input" "log" "output" "params" "resources" "threads" "wildcards"))

((wildcard
  (subscript
    value: (identifier) @label))
  (#any-of? @label "input" "log" "output" "params" "resources" "threads" "wildcards"))

; directive labels in block context (eg. within 'run:')
((identifier) @label
  (#any-of? @label "input" "log" "output" "params" "resources" "threads" "wildcards"))
