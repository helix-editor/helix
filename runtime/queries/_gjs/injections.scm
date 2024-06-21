; PARSE GLIMMER TEMPLATES
(call_expression
  function: [
    (identifier) @injection.language
    (member_expression
      property: (property_identifier) @injection.language)
  ]
  arguments: (template_string) @injection.content)

; e.g.: <template><SomeComponent @arg={{double @value}} /></template>
((glimmer_template) @injection.content
 (#set! injection.language "hbs"))

; Parse Ember/Glimmer/Handlebars/HTMLBars/etc. template literals
; e.g.: await render(hbs`<SomeComponent />`)
(call_expression
  function: ((identifier) @_name
             (#eq? @_name "hbs"))
  arguments: ((template_string) @glimmer
              (#offset! @glimmer 0 1 0 -1)))
