; extends

; Support injections for the code macro
(macro
  name: (macro_name) @_macro_name
  params: (macro_params
            (macro_param
              name: (param_name) @_param_name
              value: (param_value
                       (string
                         (string_content) @injection.language))))
  (macro_content) @injection.content
  (#match? @_macro_name "^code$")
  (#match? @_param_name "^language$"))

; Support the HTML macro
(macro
  name: (macro_name) @_macro_name
  (macro_content) @injection.content
  (#match? @_macro_name "^html$")
  (#set! injection.language "html"))

; Support the Python macro
(macro
  name: (macro_name) @_macro_name
  (macro_content) @injection.content
  (#match? @_macro_name "^python$")
  (#set! injection.language "python"))

; Support the Ruby macro
(macro
  name: (macro_name) @_macro_name
  (macro_content) @injection.content
  (#match? @_macro_name "^ruby$")
  (#set! injection.language "ruby"))

; Support the Groovy macro
(macro
  name: (macro_name) @_macro_name
  (macro_content) @injection.content
  (#match? @_macro_name "^groovy$")
  (#set! injection.language "^groovy"))

; Support the Velocity macro
(macro
  name: (macro_name) @_macro_name
  (macro_content) @injection.content
  (#match? @_macro_name "^velocity$")
  (#set! injection.language "^velocity"))

; Support the PHP macro
(macro
  name: (macro_name) @_macro_name
  (macro_content) @injection.content
  (#match? @_macro_name "^php$")
  (#set! injection.language "php"))
