; Specific parametric macro expansion rules (must come first)
(macro_expansion_call
  (builtin) @function.macro)

(macro_expansion_call
  (identifier) @function.macro)

(macro_expansion_call
  option: (macro_option) @variable.parameter
  argument: [
    (word) @variable.parameter
    (concatenation
      (word) @variable.parameter)
    (macro_expansion) @function.call
    (macro_simple_expansion) @function.call
  ])

; Highlight macro options in parametric expansions
(macro_option) @variable.parameter

; Macro expansion rules
(macro_expansion
  (builtin) @variable.builtin
  argument: (_) @variable.parameter)

(macro_expansion
  (identifier) @function.call
  argument: [
    (word) @variable.parameter
    (concatenation
      (word) @variable.parameter)
  ])

; Macro definition and undefinition
(macro_definition
  "%" @punctuation.special
  (builtin) @keyword.directive.define
  (identifier) @keyword.macro)
(macro_undefinition
  (builtin) @keyword.directive.define
  (identifier) @keyword.macro)

; General punctuation for macros
(macro_simple_expansion
  "%" @punctuation.special) @none
(macro_expansion
  "%{" @punctuation.special
  "}" @punctuation.special) @none

; General identifier and builtin rules (must come after specific rules)
(special_variable_name) @constant
(builtin) @variable.builtin

(setup_macro
  argument: [
    (setup_flag) @variable.parameter
    (setup_source_option) @variable.parameter
    ((setup_name_option
      directory: (_) @string) @variable.parameter)
  ])

(patch_macro
  [
    (patch_flag) @variable.parameter
    (patch_number_option) @variable.parameter
    (patch_string_option) @variable.parameter
    (patch_long_option) @variable.parameter
  ])

[
  (tag)
  (dependency_tag)
] @type.definition

(integer) @constant.numeric.integer
(float) @constant.numeric.float
(version) @constant.numeric.float

(comment) @comment
;(string) @string
(quoted_string) @string

(description
  (section_name) @type.definition)
(package
  (section_name) @type.definition)
(files
  (section_name) @type.definition)
(changelog
  (section_name) @type.definition)

(prep_scriptlet
  (section_name) @function.builtin)
(generate_buildrequires
  (section_name) @function.builtin)
(conf_scriptlet
  (section_name) @function.builtin)
(build_scriptlet
  (section_name) @function.builtin)
(install_scriptlet
  (section_name) @function.builtin)
(check_scriptlet
  (section_name) @function.builtin)
(clean_scriptlet
  (section_name) @function.builtin)

[
  "%pre"
  "%post"
  "%preun"
  "%postun"
  "%pretrans"
  "%posttrans"
  "%preuntrans"
  "%postuntrans"
  "%verify"
] @function.builtin

[
  "%triggerprein"
  "%triggerin"
  "%triggerun"
  "%triggerpostun"
] @function.builtin

[
  "%filetriggerin"
  "%filetriggerun"
  "%filetriggerpostun"
  "%transfiletriggerin"
  "%transfiletriggerun"
  "%transfiletriggerpostun"
] @function.builtin

[
  "%artifact"
  "%attr"
  "%config"
  "%dir"
  "%doc"
  "%docdir"
  "%ghost"
  "%license"
  "%missingok"
  "%readme"
] @keyword.type

[
  "!="
  "<"
  "<="
  "=="
  ">"
  ">="
  "and"
  "&&"
  "or"
  "||"
] @operator

[
  "with"
  "without"
  "defined"
  "undefined"
] @keyword.operator

[
  "%if"
  "%ifarch"
  "%ifos"
  "%ifnarch"
  "%ifnos"
  "%elif"
  "%elifarch"
  "%elifos"
  "%else"
  "%endif"
] @keyword.conditional

; Fallback rule for identifiers (commented out due to conflicts with parametric macros)
; (identifier) @variable
